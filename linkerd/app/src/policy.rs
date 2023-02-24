use linkerd_app_core::{
    control, dns,
    exp_backoff::{ExponentialBackoff, ExponentialBackoffStream},
    identity, metrics,
    profiles::{self, DiscoveryRejected},
    proxy::{api_resolve as api, http, resolve::recover},
    svc::{self, BoxCloneService, NewService, ServiceExt},
    Error, Recover,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub control: control::Config,
    pub workload: String,
}

/// Handles to policy service clients.
pub struct Policy<S> {
    /// The address of the policy service, used for logging.
    pub addr: control::ControlAddr,

    /// Policy service gRPC client.
    pub client: S,

    /// Workload identifier
    pub workload: Arc<str>,
}

// === impl Config ===

impl Config {
    pub fn build(
        self,
        dns: dns::Resolver,
        metrics: metrics::ControlHttp,
        identity: identity::NewClient,
    ) -> Result<
        Policy<
            impl svc::Service<
                    http::Request<tonic::body::BoxBody>,
                    Response = http::Response<control::RspBody>,
                    Error = Error,
                    Future = impl Send,
                > + Clone,
        >,
        Error,
    > {
        let addr = self.control.addr.clone();
        let workload = self.workload.into();
        let client = self
            .control
            .build(dns, metrics, identity)
            .new_service(())
            .map_err(Error::from);

        Ok(Policy {
            addr,
            client,
            workload,
        })
    }
}
