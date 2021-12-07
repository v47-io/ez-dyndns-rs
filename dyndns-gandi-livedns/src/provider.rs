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

use dyndns::config::Config;
use dyndns::provider::{DnsProvider, DnsZones, Record, Zone};
use dyndns::result::DynResult;
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use crate::client::model::*;
use crate::client::LDClient;

#[derive(Default)]
pub struct GandiLivednsProvider {
    client: LDClient,
}

impl DnsProvider for GandiLivednsProvider {
    fn current(&self, config: &Config) -> DynResult<DnsZones> {
        let mut zones = HashMap::new();

        let domains = self.client.get_domains()?;
        for domain in domains {
            if !config.zones.contains_key(&domain.fqdn) {
                continue;
            }

            let records = self.client.get_records(&domain.fqdn)?;

            zones.insert(
                Zone::new(domain.fqdn.clone()),
                records
                    .into_iter()
                    .filter_map(|record| {
                        let record_name = record.proper_name(&domain.fqdn);

                        match record.r#type {
                            LDRecordType::A => Some(Record::A {
                                name: record_name,
                                value: Ipv4Addr::from_str(record.values.first()?).unwrap(),
                                ttl: record.ttl,
                            }),
                            LDRecordType::Aaaa => Some(Record::AAAA {
                                name: record_name,
                                value: Ipv6Addr::from_str(record.values.first()?).unwrap(),
                                ttl: record.ttl,
                            }),
                        }
                    })
                    .collect(),
            );
        }

        Ok(zones)
    }

    fn update(&self, zone: &Zone, record: Record) -> DynResult<()> {
        todo!()
    }
}

trait ProperRecord {
    fn proper_name(&self, domain_name: &str) -> String;
}

impl ProperRecord for LDRecord {
    fn proper_name(&self, domain_name: &str) -> String {
        match self.name.as_ref() {
            "@" => domain_name.into(),
            name => name.to_owned() + "." + domain_name,
        }
    }
}
