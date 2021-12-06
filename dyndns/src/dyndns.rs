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

use std::net::{Ipv4Addr, Ipv6Addr};
use std::process::exit;
use std::rc::Rc;
use std::sync::Mutex;

use anyhow::Context;
use chrono::Local;

use crate::config::Config;
use crate::ip::{get_ip, Ip};
use crate::job::start_job;
use crate::provider::{DnsProvider, DnsZones, Record, Zone};
use crate::result::DynResult;

pub fn run<P: DnsProvider>(config: &Config, provider: &P) {
    let failure_count = Rc::new(Mutex::new(0));

    start_job(config, || {
        let mut failure_count = failure_count.lock().unwrap();

        if let Err(err) = run_once(config, provider) {
            eprintln!("{:?}", err);
            *failure_count += 1;
        } else {
            *failure_count = 0;
        }

        if *failure_count >= 3 {
            eprintln!("Too many errors in sequence: Aborting!");
            exit(1);
        }
    });
}

pub fn run_once<P: DnsProvider>(config: &Config, provider: &P) -> DynResult<()> {
    println!("Updating DNS records at {}", Local::now());

    let current_ip = get_ip().context("failed to retrieve external IP address")?;

    println!("Detected external IP address: {}", current_ip);

    let current_zones = provider
        .current(config)
        .context("failed to retrieve current DNS data")?;

    config.zones.iter().for_each(|(zone, records)| {
        println!("---");
        println!("Zone: {}", zone);

        records.iter().for_each(|record| match &current_ip {
            Ip::V4(ipv4) => update_a_record(
                provider,
                zone.as_str(),
                record.a.as_deref(),
                ipv4,
                &current_zones,
            ),
            Ip::V6(ipv6) => update_aaaa_record(
                provider,
                zone.as_str(),
                record.aaaa.as_deref(),
                ipv6,
                &current_zones,
            ),
        });
    });

    println!("---");
    println!("Done updating DNS records at {}", Local::now());

    Ok(())
}

fn update_a_record<P: DnsProvider>(
    provider: &P,
    zone: &str,
    a_record: Option<&str>,
    address: &Ipv4Addr,
    current_zones: &DnsZones,
) {
    let a_record = if let Some(a_record) = a_record {
        a_record
    } else {
        return;
    };

    let zone = current_zones.find_or_create(zone);

    let current_record = current_zones
        .iter()
        .find(|(zone_id, _)| zone_id == &&zone)
        .map(|(_, zone_content)| {
            zone_content.iter().find(|&record| {
                if let Record::A { name, .. } = record {
                    name == a_record
                } else {
                    false
                }
            })
        });

    let current_value = if let Some(Some(record)) = current_record {
        match record {
            Record::A { value, .. } => Some(value),
            _ => panic!(),
        }
    } else {
        None
    };

    let new_record = Record::A {
        name: a_record.to_string(),
        value: *address,
    };

    if let Some(current_value) = current_value {
        if current_value != address {
            println!(
                "Updating A record {}: {} => {}",
                a_record, current_value, address
            );
            wrap_update(provider, &zone, new_record)
        } else {
            println!("Not updating {}: Unchanged", a_record);
        }
    } else {
        println!("Creating A record {}: {}", a_record, address);
        wrap_update(provider, &zone, new_record)
    }
}

fn update_aaaa_record<P: DnsProvider>(
    provider: &P,
    zone: &str,
    aaaa_record: Option<&str>,
    address: &Ipv6Addr,
    current_zones: &DnsZones,
) {
    let aaaa_record = if let Some(aaaa_record) = aaaa_record {
        aaaa_record
    } else {
        return;
    };

    let zone = current_zones.find_or_create(zone);

    let current_record = current_zones
        .iter()
        .find(|(zone_id, _)| zone_id == &&zone)
        .map(|(_, zone_content)| {
            zone_content.iter().find(|&record| {
                if let Record::AAAA { name, .. } = record {
                    name == aaaa_record
                } else {
                    false
                }
            })
        });

    let current_value = if let Some(Some(record)) = current_record {
        match record {
            Record::AAAA { value, .. } => Some(value),
            _ => panic!(),
        }
    } else {
        None
    };

    let new_record = Record::AAAA {
        name: aaaa_record.to_string(),
        value: *address,
    };

    if let Some(current_value) = current_value {
        if current_value != address {
            println!(
                "Updating AAAA record {}: {} => {}",
                aaaa_record, current_value, address
            );
            wrap_update(provider, &zone, new_record)
        } else {
            println!("Not updating {}: Unchanged", aaaa_record);
        }
    } else {
        println!("Creating AAAA record {}: {}", aaaa_record, address);
        wrap_update(provider, &zone, new_record)
    }
}

fn wrap_update<P: DnsProvider>(provider: &P, zone: &Zone, record: Record) {
    let result = provider.update(zone, record.clone());

    if let Err(err) = result {
        eprintln!(
            "{:?}",
            err.context(format!("failed to update record {}", record))
        )
    }
}

trait FindOrCreateZone {
    fn find_or_create(&self, zone: &str) -> Zone;
}

impl FindOrCreateZone for DnsZones {
    fn find_or_create(&self, zone: &str) -> Zone {
        self.keys()
            .find(|key| key.name == zone)
            .cloned()
            .unwrap_or_else(|| Zone::new(zone.into()))
    }
}
