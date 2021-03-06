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
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::config::Config;
use crate::result::DynResult;

pub type DnsZones = HashMap<Zone, DnsRecords>;

pub type DnsRecords = Vec<Record>;

pub trait DnsProvider {
    fn current(&self, config: &Config) -> DynResult<DnsZones>;

    fn update(&self, zone: &Zone, record: Record) -> DynResult<()>;
}

#[derive(Clone, Debug, Eq)]
pub struct Zone {
    pub name: String,
    pub id: Option<String>,
}

impl Zone {
    pub fn new(name: String) -> Zone {
        Zone { name, id: None }
    }

    pub fn with_id(name: String, id: String) -> Zone {
        Zone { name, id: Some(id) }
    }
}

impl Hash for Zone {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl PartialEq for Zone {
    fn eq(&self, other: &Self) -> bool {
        other.name == self.name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Record {
    A {
        name: String,
        value: Ipv4Addr,
        ttl: u32,
    },
    AAAA {
        name: String,
        value: Ipv6Addr,
        ttl: u32,
    },
}

impl Display for Record {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Record::A { name, value, .. } => write!(f, "(A {}): {}", name, value),
            Record::AAAA { name, value, .. } => write!(f, "(AAAA {}): {}", name, value),
        }
    }
}
