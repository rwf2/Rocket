# Deployment

Rocket can be deployed to production in a few different ways:

1. On fully managed platforms like [Render](https://render.com) with native Rust support.

2. Directly on a VM using a cloud provider like [AWS](https://aws.amazon.com).

3. As a [Docker](https://docs.docker.com) container on [Kubernetes](https://kubernetes.io).


## Render

Render is a modern cloud platform that offers native support for Rust, fully managed SSL, custom domains, managed databases, zero-downtime deploys, and websocket support.

Render integrates directly with GitHub to automatically build and deploy your app on every push. It supports multiple Rust toolchains including `nightly`.

Assuming you have a `Cargo.toml` at the root of you repo, going live on Render takes just a few clicks:

1. Add a file called `rust-toolchain` and the root of your repo, and add valid nightly version to it. You can use the latest nightly version like so:
    ```text
    nightly
    ```

    You can also pin the nightly version for your app by specifying a date as follows:

    ```text
    nightly-06-10-2019
    ```

    Render will automatically detect and install the toolchain specified in this file. Commit and push your changes before proceeding to the next step.

2. Create a new **Web Service** on Render, and give Render's GitHub app permission to access your Rocket repository.

3. Use the following values during creation:

   |            |           |
   | ---------- | --------- |
   | **Environment** | `Rust` |
   | **Build Command** | `cargo build --release` |
   | **Start Command** | `cargo run --release` |

That's it! Your web service will be live on your Render URL as soon as the build finishes.

Render has additional guides for deploying Rocket apps on their platform:

* [Deploying a Rust Web App with Rocket](https://render.com/docs/deploy-rocket-rust)
* [Deploy a Rust GraphQL Server with Juniper
](https://render.com/docs/deploy-rust-graphql)

Learn more in [Render docs](https:/render.com/docs).
