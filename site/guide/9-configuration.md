# Configuration

Rocket aims to have a flexible and usable configuration system. Rocket
applications can be configured via a configuration file, through environment
variables, or both. Configurations are separated into three environments:
development, staging, and production. The working environment is selected via an
environment variable.

## Environment

At any point in time, a Rocket application is operating in a given
_configuration environment_. There are three such environments:

   * `development` (short: `dev`)
   * `staging` (short: `stage`)
   * `production` (short: `prod`)

Without any action, Rocket applications run in the `development` environment for
debug builds and the `production` environment for non-debug builds. The
environment can be changed via the `ROCKET_ENV` environment variable. For
example, to launch an application in the `staging` environment, we can run:

```sh
ROCKET_ENV=stage cargo run
```

Note that you can use the short or long form of the environment name to specify
the environment, `stage` _or_ `staging` here. Rocket tells us the environment we
have chosen and its configuration when it launches:

```sh
$ sudo ROCKET_ENV=staging cargo run

🔧  Configured for staging.
    => address: 0.0.0.0
    => port: 8000
    => log: normal
    => workers: 24
    => secret key: generated
    => limits: forms = 32KiB
    => keep-alive: 5s
    => tls: disabled
🛰  Mounting '/':
    => GET / (hello)
🚀  Rocket has launched from http://0.0.0.0:8000
```

## Rocket.toml

An optional `Rocket.toml` file can be used to specify the configuration
parameters for each environment. If it is not present, the default configuration
parameters are used. Rocket searches for the file starting at the current
working directory. If it is not found there, Rocket checks the parent directory.
Rocket continues checking parent directories until the root is reached.

The file must be a series of TOML tables, at most one for each environment, and
an optional "global" table. Each table contains key-value pairs corresponding to
configuration parameters for that environment. If a configuration parameter is
missing, the default value is used. The following is a complete `Rocket.toml`
file, where every standard configuration parameter is specified with the default
value:

```toml
[development]
address = "localhost"
port = 8000
workers = [number of cpus * 2]
keep_alive = 5
log = "normal"
secret_key = [randomly generated at launch]
limits = { forms = 32768 }

[staging]
address = "0.0.0.0"
port = 8000
workers = [number of cpus * 2]
keep_alive = 5
log = "normal"
secret_key = [randomly generated at launch]
limits = { forms = 32768 }

[production]
address = "0.0.0.0"
port = 8000
workers = [number of cpus * 2]
keep_alive = 5
log = "critical"
secret_key = [randomly generated at launch]
limits = { forms = 32768 }
```

The `workers` and `secret_key` default parameters are computed by Rocket
automatically; the values above are not valid TOML syntax. When manually
specifying the number of workers, the value should be an integer: `workers =
10`. When manually specifying the secret key, the value should a 256-bit base64
encoded string. Such a string can be generated using a tool such as openssl:
`openssl rand -base64 32`.

The "global" pseudo-environment can be used to set and/or override configuration
parameters globally. A parameter defined in a `[global]` table sets, or
overrides if already present, that parameter in every environment. For example,
given the following `Rocket.toml` file, the value of `address` will be
`"1.2.3.4"` in every environment:

```
[global]
address = "1.2.3.4"

[development]
address = "localhost"

[production]
address = "0.0.0.0"
```

## Data Limits

The `limits` parameter configures the maximum amount of data Rocket will accept
for a given data type. The parameter is a table where each key corresponds to a
data type and each value corresponds to the maximum size in bytes Rocket
should accept for that type.

By default, Rocket limits forms to 32KiB (32768 bytes). To increase the limit,
simply set the `limits.forms` configuration parameter. For example, to increase
the forms limit to 128KiB globally, we might write:

```toml
[global.limits]
forms = 131072
```

The `limits` parameter can contain keys and values that are not endemic to
Rocket. For instance, the [`Json`] type reads the `json` limit value to cap
incoming JSON data. You should use the `limits` parameter for your application's
data limits as well. Data limits can be retrieved at runtime via the
[`Request::limits()`] method.

[`Request::limits()`]: @api/rocket/struct.Request.html#method.limits
[`Json`]: @api/rocket_contrib/json/struct.Json.html#incoming-data-limits

## Extras

In addition to overriding default configuration parameters, a configuration file
can also define values for any number of _extra_ configuration parameters. While
these parameters aren't used by Rocket directly, other libraries, or your own
application, can use them as they wish. As an example, the
[Template](@api/rocket_contrib/templates/struct.Template.html) type
accepts a value for the `template_dir` configuration parameter. The parameter
can be set in `Rocket.toml` as follows:

```toml
[development]
template_dir = "dev_templates/"

[production]
template_dir = "prod_templates/"
```

This sets the `template_dir` extra configuration parameter to `"dev_templates/"`
when operating in the `development` environment and `"prod_templates/"` when
operating in the `production` environment. Rocket will prepend the `[extra]` tag
to extra configuration parameters when launching:

```sh
🔧  Configured for development.
    => ...
    => [extra] template_dir: "dev_templates/"
```

To retrieve a custom, extra configuration parameter in your application, we
recommend using an [ad-hoc attach fairing] in combination with [managed state].
For example, if your application makes use of a custom `assets_dir` parameter:

[ad-hoc attach fairing]: ../fairings/#ad-hoc-fairings
[managed state]: ../state/#managed-state

```toml
[development]
assets_dir = "dev_assets/"

[production]
assets_dir = "prod_assets/"
```

The following code will:

  1. Read the configuration parameter in an ad-hoc `attach` fairing.
  2. Store the parsed parameter in an `AssertsDir` structure in managed state.
  3. Retrieve the parameter in an `assets` route via the `State` guard.

```rust
struct AssetsDir(String);

#[get("/<asset..>")]
fn assets(asset: PathBuf, assets_dir: State<AssetsDir>) -> Option<NamedFile> {
    NamedFile::open(Path::new(&assets_dir.0).join(asset)).ok()
}

fn main() {
    rocket::ignite()
        .mount("/", routes![assets])
        .attach(AdHoc::on_attach("Assets Config", |rocket| {
            let assets_dir = rocket.config()
                .get_str("assets_dir")
                .unwrap_or("assets/")
                .to_string();

            Ok(rocket.manage(AssetsDir(assets_dir)))
        }))
        .launch();
}
```

## Environment Variables

All configuration parameters, including extras, can be overridden through
environment variables. To override the configuration parameter `{param}`, use an
environment variable named `ROCKET_{PARAM}`. For instance, to override the
"port" configuration parameter, you can run your application with:

```sh
ROCKET_PORT=3721 ./your_application

🔧  Configured for development.
    => ...
    => port: 3721
```

Environment variables take precedence over all other configuration methods: if
the variable is set, it will be used as the value for the parameter. Variable
values are parsed as if they were TOML syntax. As illustration, consider the
following examples:

```sh
ROCKET_INTEGER=1
ROCKET_FLOAT=3.14
ROCKET_STRING=Hello
ROCKET_STRING="Hello"
ROCKET_BOOL=true
ROCKET_ARRAY=[1,"b",3.14]
ROCKET_DICT={key="abc",val=123}
```

## Programmatic

In addition to using environment variables or a config file, Rocket can also be
configured using the [`rocket::custom()`] method and [`ConfigBuilder`]:

```rust
use rocket::config::{Config, Environment};

let config = Config::build(Environment::Staging)
    .address("1.2.3.4")
    .port(9234)
    .finalize()?;

let app = rocket::custom(config, false);
```

Configuration via `rocket::custom()` replaces all configuration from
`Rocket.toml` or environment variables. In other words, using `rocket::custom()`
results in `Rocket.toml` and environment variables being ignored.

[`rocket::custom()`]: @api/rocket/fn.custom.html
[`ConfigBuilder`]: @api/rocket/config/struct.ConfigBuilder.html

## Configuring TLS

! warning: Rocket's built-in TLS is **not** considered ready for production use.
  It is intended for development use _only_.

Rocket includes built-in, native support for TLS >= 1.2 (Transport Layer
Security). In order for TLS support to be enabled, Rocket must be compiled with
the `"tls"` feature. To do this, add the `"tls"` feature to the `rocket`
dependency in your `Cargo.toml` file:

```
[dependencies]
rocket = { version = "0.4.0-dev", features = ["tls"] }
```

TLS is configured through the `tls` configuration parameter. The value of `tls`
must be a table with two keys:

  * `certs`: _[string]_ a path to a certificate chain in PEM format
  * `key`: _[string]_ a path to a private key file in PEM format for the
    certificate in `certs`

The recommended way to specify these parameters is via the `global` environment:

```
[global.tls]
certs = "/path/to/certs.pem"
key = "/path/to/key.pem"
```

Of course, you can always specify the configuration values per environment:

```
[development]
tls = { certs = "/path/to/certs.pem", key = "/path/to/key.pem" }
```

Or via environment variables:

```sh
ROCKET_TLS={certs="/path/to/certs.pem",key="/path/to/key.pem"} cargo run
```
