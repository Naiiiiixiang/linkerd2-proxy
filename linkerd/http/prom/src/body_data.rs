use linkerd_metrics::prom;
use linkerd_stack as svc;

pub struct NewRecordBodyData<X, N> {
    extract: X,
    inner: N,
}

#[derive(Clone, Debug, Default)]
pub struct BodyDataMetrics();

// === impl NewRecordBodyData ===

impl<X: Clone, N> NewRecordBodyData<X, N> {
    /// Returns a [`Layer<S>`][svc::layer::Layer] that tracks body chunks.
    ///
    /// This uses an `X`-typed [`ExtractParam<P, T>`][svc::ExtractParam] implementation to extract
    /// service parameters from a `T`-typed target.
    pub fn layer_via(extract: X) -> impl svc::layer::Layer<N, Service = Self> {
        svc::layer::mk(move |inner| Self {
            extract: extract.clone(),
            inner,
        })
    }
}

// === impl BodyDataMetrics ===

impl BodyDataMetrics {
    pub fn register(_registry: &mut prom::Registry) -> Self {
        // DEV(kate); register metrics with prometheus here.
        Self()
    }
}
