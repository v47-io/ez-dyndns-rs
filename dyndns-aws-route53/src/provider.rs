/*
 * BSD 3-Clause License
 *
 * Copyright (c) 2021, Alex Katlein
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 */

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_route53::model::{
    Change, ChangeAction, ChangeBatch, HostedZone, ResourceRecord, ResourceRecordSet, RrType,
};
use aws_sdk_route53::{Client, Region};
use dyndns::config::Config;
use dyndns::provider::{DnsProvider, DnsRecords, DnsZones, Record, Zone};
use dyndns::result::DynResult;
use std::collections::HashMap;
use std::error::Error;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ops::Add;
use std::rc::Rc;
use std::str::FromStr;
use tokio::runtime::Runtime;

pub struct AwsRoute53Provider {
    runtime: Rc<Runtime>,
    client: Client,
    _config: aws_config::Config,
}

impl Default for AwsRoute53Provider {
    fn default() -> Self {
        AwsRoute53Provider::with_runtime(Rc::new(Runtime::new().unwrap()))
    }
}

impl AwsRoute53Provider {
    pub fn with_runtime(runtime: Rc<Runtime>) -> Self {
        let build_instance = async {
            let region_provider =
                RegionProviderChain::default_provider().or_else(Region::new("us-east-1"));

            let config = aws_config::from_env().region(region_provider).load().await;
            let client = Client::new(&config);

            AwsRoute53Provider {
                runtime: Rc::clone(&runtime),
                client,
                _config: config,
            }
        };

        runtime.block_on(build_instance)
    }
}

impl DnsProvider for AwsRoute53Provider {
    fn current(&self, config: &Config) -> DynResult<DnsZones> {
        self.runtime.block_on(current(self, config))
    }

    fn update(&self, zone: &Zone, record: Record) -> DynResult<()> {
        self.runtime.block_on(update(self, zone, record))
    }
}

async fn current(provider: &AwsRoute53Provider, config: &Config) -> DynResult<DnsZones> {
    let mut aws_zones: Vec<HostedZone> = Vec::new();

    let mut last_aws_zone_marker = None;
    loop {
        let aws_list_zones_request = provider
            .client
            .list_hosted_zones()
            .set_marker(last_aws_zone_marker.clone());

        match aws_list_zones_request.send().await {
            Ok(aws_output) => {
                aws_zones.append(
                    &mut aws_output
                        .hosted_zones
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|hosted_zone| {
                            if let Some(hz_name) = &hosted_zone.name {
                                config.zones.contains_key(hz_name.as_internal())
                            } else {
                                false
                            }
                        })
                        .collect(),
                );

                // aws_zones.append(&mut aws_output.hosted_zones.unwrap_or_default());
                if aws_output.is_truncated {
                    last_aws_zone_marker = aws_output.marker
                } else {
                    break;
                }
            }
            Err(err) => return Err(dyndns::anyhow::Error::from(err)),
        }
    }

    let mut result = HashMap::new();
    for aws_zone in aws_zones {
        let mut dns_records: DnsRecords = Vec::new();

        let aws_zone_id = aws_zone.zone_id();
        let aws_zone_name = aws_zone.name.unwrap();
        let configured_zone = config.zones.get(aws_zone_name.as_internal()).unwrap();

        let mut last_aws_record_identifier = None;
        loop {
            let aws_list_records_request = provider
                .client
                .list_resource_record_sets()
                .hosted_zone_id(aws_zone_id.clone())
                .set_start_record_identifier(last_aws_record_identifier.clone());

            let aws_response = aws_list_records_request.send().await;

            match aws_response {
                Ok(aws_output) => {
                    dns_records.append(
                        &mut aws_output
                            .resource_record_sets
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|record_set| {
                                let record_set_name =
                                    record_set.name.unwrap().as_internal().to_string();

                                let record_set_type = record_set.r#type.unwrap();

                                configured_zone
                                    .iter()
                                    .find(|record| match record_set_type {
                                        RrType::A => {
                                            if let Some(a_record) = record.a.as_ref() {
                                                a_record == &record_set_name
                                            } else {
                                                false
                                            }
                                        }
                                        RrType::Aaaa => {
                                            if let Some(aaaa_record) = record.aaaa.as_ref() {
                                                aaaa_record == &record_set_name
                                            } else {
                                                false
                                            }
                                        }
                                        _ => false,
                                    })?;

                                let records = record_set.resource_records.unwrap_or_default();
                                if !records.is_empty() {
                                    match record_set_type {
                                        RrType::A => Some(Record::A {
                                            name: record_set_name,
                                            value: Ipv4Addr::from_str(
                                                if let Some(Some(value)) =
                                                    records.first().map(|it| &it.value)
                                                {
                                                    value
                                                } else {
                                                    return None;
                                                },
                                            )
                                            .unwrap(),
                                            ttl: record_set.ttl.unwrap().try_into().unwrap(),
                                        }),
                                        RrType::Aaaa => Some(Record::AAAA {
                                            name: record_set_name,
                                            value: Ipv6Addr::from_str(
                                                if let Some(Some(value)) =
                                                    records.first().map(|it| &it.value)
                                                {
                                                    value
                                                } else {
                                                    return None;
                                                },
                                            )
                                            .unwrap(),
                                            ttl: record_set.ttl.unwrap().try_into().unwrap(),
                                        }),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    );

                    if aws_output.is_truncated {
                        last_aws_record_identifier = aws_output.next_record_identifier
                    } else {
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("{:?}", err);
                    eprintln!("{:?}", err.source());

                    return Err(dyndns::anyhow::Error::from(err));
                }
            }
        }

        result.insert(
            Zone::with_id(aws_zone_name.as_internal().into(), aws_zone_id),
            dns_records,
        );
    }

    Ok(result)
}

async fn update(provider: &AwsRoute53Provider, zone: &Zone, record: Record) -> DynResult<()> {
    let zone_id = if let Some(zone_id) = &zone.id {
        zone_id.clone()
    } else {
        eprintln!("No such hosted zone: {}", zone.name);
        return Ok(());
    };

    provider
        .client
        .change_resource_record_sets()
        .hosted_zone_id(zone_id)
        .change_batch(
            ChangeBatch::builder()
                .changes(
                    Change::builder()
                        .action(ChangeAction::Upsert)
                        .resource_record_set(record.to_resource_record_set())
                        .build(),
                )
                .build(),
        )
        .send()
        .await?;

    Ok(())
}

trait AwsDomainName {
    fn as_internal(&self) -> &str;

    fn to_aws(&self) -> String;
}

impl AwsDomainName for String {
    fn as_internal(&self) -> &str {
        self.strip_suffix('.').unwrap_or(self)
    }

    fn to_aws(&self) -> String {
        self.as_internal().to_string().add(".")
    }
}

trait AwsHostedZone {
    fn zone_id(&self) -> String;
}

impl AwsHostedZone for HostedZone {
    fn zone_id(&self) -> String {
        self.id
            .as_ref()
            .map(|zone| {
                if let Some(i) = zone.rfind('/') {
                    zone[i + 1..].to_string()
                } else {
                    zone.to_string()
                }
            })
            .unwrap()
    }
}

trait AwsRecord {
    fn to_resource_record_set(&self) -> ResourceRecordSet;
}

impl AwsRecord for Record {
    fn to_resource_record_set(&self) -> ResourceRecordSet {
        let (name, r#type, value, ttl) = match self {
            Record::A { name, value, ttl } => (name.to_aws(), RrType::A, value.to_string(), *ttl),
            Record::AAAA { name, value, ttl } => {
                (name.to_aws(), RrType::Aaaa, value.to_string(), *ttl)
            }
        };

        ResourceRecordSet::builder()
            .name(name)
            .r#type(r#type)
            .resource_records(ResourceRecord::builder().value(value).build())
            .ttl(ttl.into())
            .build()
    }
}
