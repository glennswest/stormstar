//! Database layer — native_db Store wrapper.

pub mod models;

use native_db::*;
use once_cell::sync::Lazy;

use models::*;

/// All registered native_db models.
pub static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<Organization>().expect("Organization model");
    models.define::<Product>().expect("Product model");
    models.define::<Repository>().expect("Repository model");
    models.define::<Package>().expect("Package model");
    models.define::<Erratum>().expect("Erratum model");
    models.define::<ContentView>().expect("ContentView model");
    models.define::<ContentViewVersion>().expect("ContentViewVersion model");
    models.define::<ContentViewFilter>().expect("ContentViewFilter model");
    models.define::<LifecycleEnvironment>().expect("LifecycleEnvironment model");
    models.define::<Host>().expect("Host model");
    models.define::<ActivationKey>().expect("ActivationKey model");
    models.define::<SyncPlan>().expect("SyncPlan model");
    models.define::<HostCollection>().expect("HostCollection model");
    models
});

/// Open (or create) the native_db database at the given path.
pub fn open_db(path: &str) -> anyhow::Result<Database<'static>> {
    let db = Builder::new()
        .create(&MODELS, path)
        .map_err(|e| anyhow::anyhow!("failed to open database '{}': {e}", path))?;
    Ok(db)
}
