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

use anyhow::Error;
use std::path::{Path, PathBuf};
use std::process::exit;

use crate::config::load_config;
use crate::provider::DnsProvider;

pub fn cli<F, D: DnsProvider>(name: &str, version: &str, provider: F)
where
    F: Fn() -> D,
{
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print_help(name, version);
        exit(0);
    }

    if pargs.contains("--version") {
        println!("{} {}", name, version);
        exit(0);
    }

    let once = pargs.contains("--once");
    let config_path = match pargs.free_from_str::<PathBuf>() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{:?}", Error::from(err).context("invalid config path"));
            print_help(name, version);
            exit(1);
        }
    };

    if once {
        run_once(&config_path, provider())
    } else {
        run(&config_path, provider())
    }
}

fn print_help(name: &str, version: &str) {
    println!(
        r#"\
{name} {}
Updates DNS entries to match your external IP address

USAGE:
  {name} [FLAGS] <CONFIG>

FLAGS:
  --once                Runs the DNS update once and then quits

  -h, --h               Prints help information
  --version             Prints the version

ARGS:
  <CONFIG>              Path to the configuration file
"#,
        version,
        name = name
    )
}

pub fn run<D: DnsProvider, P: AsRef<Path>>(config_path: P, provider: D) {
    let config = match load_config(config_path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{:?}", err);
            exit(1);
        }
    };

    crate::run(&config, &provider);
}

pub fn run_once<D: DnsProvider, P: AsRef<Path>>(config_path: P, provider: D) {
    let config = match load_config(config_path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{:?}", err);
            exit(1);
        }
    };

    if let Err(err) = crate::run_once(&config, &provider) {
        eprintln!("{:?}", err);
        exit(1);
    }
}
