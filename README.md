# ez-dyndns-rs

ez-dyndns-rs is an easy-to-use dynamic DNS suite that requires minimal configuration 
and "Just Works".

It supports the following providers:
  - AWS Route 53
  - Gandi LiveDNS

The intended use-case is for people who need to access their homelab via the internet
but don't have a static IP address.

## Configuration

ez-dyndns-rs requires only a simple configuration file that identifies the domains and
records to be set to the detected external IP address (IPv4 and IPv6 are supported).

__Sample__ (YAML):

```yaml
interval: 1800 # seconds, 30 minutes per default
zones:
  testdomain.com:
    - a: '*.testdomain.com'
    - a: testdomain.com
      ttl: 600 # seconds
    - aaaa: ipv6.testdomain.com
```

ez-dyndns-rs does support IPv6 and setting AAAA records, but the actual IP address depends
on what is detected by [icanhazip](https://icanhazip.com) which only provides either IPv4
or IPv6, not both.

## Executables

Each implementation crate provides a daemon executable, e.g. `dyndnsd-gandi-livedns` which
checks for a changed external IP address per the configured interval.

To start the daemon execute it and provide the path to the configuration file via a positional 
command line argument.

If you just want to test your configuration once and then exit, simply specify the option
`--once` when running the executable.

## License and Contributions

ez-dyndns-rs is provided under the terms of the BSD 3-Clause License.

Contributions are welcome, but you must have all rights to the contributed material and agree 
to provide it under the terms of the aforementioned BSD 3-Clause License.
