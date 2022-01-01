# ez-dyndns-rs

ez-dyndns-rs is a suite of easy-to-use dynamic DNS clients that require minimal configuration and "Just Work".

It supports the following providers:

- AWS Route 53
- Gandi LiveDNS

The intended use-case is for people who need to access their homelab via the internet but don't have a static IP
address.

## Configuration

ez-dyndns-rs requires only a simple configuration file that identifies the domains and records to be set to the detected
external IP address (IPv4 and IPv6 are supported).

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

ez-dyndns-rs does support IPv6 and setting AAAA records, but the actual IP address depends on what is detected
by [icanhazip](https://icanhazip.com) which only provides either IPv4 or IPv6, not both.

## Executables

Each implementation crate provides a daemon executable, e.g. `dyndns-gandi-livedns` which checks for a changed external
IP address per the configured interval.

To start the daemon execute it and provide the path to the configuration file via a positional command line argument.

If you just want to test your configuration once and then exit, simply specify the option
`--once` when running the executable.

### Docker

There are also Docker images for each provider that can be found here: [Docker Hub][hub-v47io]

To run them provide a volume containing a configuration file and specify it when running the container:

```shell
docker run -v $(pwd):/config:ro -e LIVEDNS_API_KEY=xxx v47io/ez-dyndns-gandi-livedns /config/my-gandi-domains-config.yml
```

[hub-v47io]: https://hub.docker.com/u/v47io

### Authorization

You need to specify the following environment variables to authorize ez-dyndns-rs to perform DNS changes on your behalf.

#### AWS Route 53

You need to provide credentials that grant access to these IAM permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "VisualEditor0",
      "Effect": "Allow",
      "Action": [
        "route53:ListHostedZones",
        "route53:ChangeResourceRecordSets",
        "route53:ListResourceRecordSets"
      ],
      "Resource": "*"
    }
  ]
}
```

The best way to then grant access is to provide the necessary environment variables for AWS access:

- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`

#### Gandi LiveDNS (v5)

You need to first create a production API key on the `Security` page of your Gandi
account ([Gandi Account][gandi-account]).

Then simply specify the key using the environment variable `LIVEDNS_API_KEY`.

[gandi-account]: https://account.gandi.net

## License and Contributions

ez-dyndns-rs is provided under the terms of the BSD 3-Clause License.

Contributions are welcome, but you must have all rights to the contributed material and agree to provide it under the
terms of the aforementioned BSD 3-Clause License.
