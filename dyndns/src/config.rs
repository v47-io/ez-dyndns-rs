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

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Error};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};

use crate::result::DynResult;

const DEFAULT_INTERVAL: u64 = 1800;

const DEFAULT_TTL: u32 = 300;

#[serde_as]
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Config {
    #[serde_as(as = "DurationSeconds<u64>")]
    #[serde(default = "default_interval")]
    pub interval: Duration,
    #[serde(default = "HashMap::new")]
    pub zones: HashMap<String, Vec<DomainRecord>>,
}

fn default_interval() -> Duration {
    Duration::from_secs(DEFAULT_INTERVAL)
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct DomainRecord {
    #[serde(alias = "A")]
    pub a: Option<String>,
    #[serde(alias = "AAAA")]
    pub aaaa: Option<String>,
    #[serde(default = "default_ttl")]
    pub ttl: u32,
}

fn default_ttl() -> u32 {
    DEFAULT_TTL
}

pub fn load_config<P: AsRef<Path>>(source: P) -> DynResult<Config> {
    println!("Loading configuration file: {}", source.as_ref().display());

    let f = File::open(source).context("failed to open config file")?;
    let config: Config = serde_yaml::from_reader(f).context("failed to read config file")?;

    let zones = config
        .zones
        .into_iter()
        .filter_map(|(key, records)| {
            let records = records
                .into_iter()
                .filter(|record| !(record.a.is_none() && record.aaaa.is_none()))
                .collect::<Vec<_>>();

            if records.is_empty() {
                None
            } else {
                Some((key, records))
            }
        })
        .collect::<HashMap<_, _>>();

    if !zones.is_empty() {
        Ok(Config {
            interval: if config.interval.is_zero() {
                default_interval()
            } else {
                config.interval
            },
            zones,
        })
    } else {
        Err(Error::msg("config is empty"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_config() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(
            r#"---
zones:
  test.com:
    - a: '*.test.com'
    - a: test.com
"#
            .as_bytes(),
        )
        .unwrap();

        let config = load_config(file).unwrap();

        let mut zones = HashMap::new();
        zones.insert(
            "test.com".into(),
            vec![
                DomainRecord {
                    a: Some("*.test.com".into()),
                    aaaa: None,
                    ttl: default_ttl(),
                },
                DomainRecord {
                    a: Some("test.com".into()),
                    aaaa: None,
                    ttl: default_ttl(),
                },
            ],
        );

        assert_eq!(
            Config {
                interval: default_interval(),
                zones
            },
            config
        )
    }
}
