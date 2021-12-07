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

use dyndns::anyhow::{Context, Error};
use dyndns::result::DynResult;
use dyndns::ureq;
use std::env;

use crate::client::model::*;

static BASE_URL: &str = "https://api.gandi.net/v5/livedns";
static PER_PAGE_VALUE: &str = "2147483647";

pub(crate) struct LDClient {
    api_key: Option<String>,
}

impl Default for LDClient {
    fn default() -> Self {
        LDClient {
            api_key: env::var("LIVEDNS_API_KEY").ok(),
        }
    }
}

impl LDClient {
    pub(crate) fn get_domains(&self) -> DynResult<Vec<LDDomain>> {
        ureq::get(&format!("{}/domains", BASE_URL))
            .query("per_page", PER_PAGE_VALUE)
            .set("Authorization", &format!("Apikey {}", self.api_key()?))
            .call()
            .context("failed to call LiveDNS")?
            .into_json()
            .context("failed to read domains response")
    }

    pub(crate) fn get_records(&self, domain: &str) -> DynResult<Vec<LDRecord>> {
        let mut a_records: Vec<LDRecord> = self.get_records_for_type(domain, LDRecordType::A)?;
        let mut aaaa_records: Vec<LDRecord> =
            self.get_records_for_type(domain, LDRecordType::Aaaa)?;

        a_records.append(&mut aaaa_records);

        Ok(a_records)
    }

    fn get_records_for_type(
        &self,
        domain: &str,
        record_type: LDRecordType,
    ) -> DynResult<Vec<LDRecord>> {
        let record_type_str: &str = record_type.into();

        ureq::get(&format!("{}/domains/{}/records", BASE_URL, domain))
            .query("rrset_type", record_type_str)
            .query("per_page", PER_PAGE_VALUE)
            .set("Authorization", &format!("Apikey {}", self.api_key()?))
            .call()
            .context("failed to call LiveDNS")?
            .into_json()
            .context(format!(
                "failed to read domain {} records response",
                record_type_str
            ))
    }

    fn api_key(&self) -> DynResult<&str> {
        match &self.api_key {
            Some(api_key) => Ok(api_key.as_str()),
            _ => Err(Error::msg("Gandi LiveDNS API Key not configured")),
        }
    }
}

pub(crate) mod model {
    use serde::{Deserialize, Serialize};
    use std::fmt::{Display, Formatter};

    #[derive(Deserialize, Serialize)]
    pub struct LDDomain {
        pub fqdn: String,
    }

    #[derive(Deserialize, Serialize)]
    pub struct LDRecord {
        #[serde(rename = "rrset_type")]
        pub r#type: LDRecordType,
        #[serde(rename = "rrset_ttl")]
        pub ttl: u32,
        #[serde(rename = "rrset_name")]
        pub name: String,
        #[serde(rename = "rrset_values")]
        pub values: Vec<String>,
    }

    #[derive(Deserialize, Serialize)]
    pub enum LDRecordType {
        A,
        #[serde(rename = "AAAA")]
        Aaaa,
    }

    impl Display for LDRecordType {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    LDRecordType::A => "A",
                    LDRecordType::Aaaa => "AAAA",
                }
            )
        }
    }

    impl From<LDRecordType> for &str {
        fn from(t: LDRecordType) -> Self {
            match t {
                LDRecordType::A => "A",
                LDRecordType::Aaaa => "AAAA",
            }
        }
    }
}
