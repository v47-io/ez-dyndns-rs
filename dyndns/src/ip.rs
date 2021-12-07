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

use crate::result::DynResult;
use anyhow::Context;
use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

pub(crate) enum Ip {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

impl Display for Ip {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ip::V4(ipv4) => write!(f, "(IPv4 {})", ipv4),
            Ip::V6(ipv6) => write!(f, "(IPv6 {})", ipv6),
        }
    }
}

pub(crate) fn get_ip() -> DynResult<Ip> {
    let raw_ip = ureq::get("https://icanhazip.com")
        .call()
        .context("failed to reach icanhazip.com")?
        .into_string()
        .context("failed to decode response")?;

    let trimmed_ip = raw_ip.trim();

    Ok(if trimmed_ip.contains(':') {
        Ip::V6(
            Ipv6Addr::from_str(trimmed_ip)
                .context(format!("failed to parse IPv6: {}", trimmed_ip))?,
        )
    } else {
        Ip::V4(
            Ipv4Addr::from_str(trimmed_ip)
                .context(format!("failed to parse IPv4: {}", trimmed_ip))?,
        )
    })
}
