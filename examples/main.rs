#[macro_use]
extern crate rocket;

use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use rocket_prometheus::PrometheusMetrics;

type NameCounter = Family<NameLabel, Counter>;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct NameLabel {
    name: String,
}

mod routes {
    use rocket::serde::json::Json;
    use rocket::State;
    use serde::Deserialize;

    use super::{NameCounter, NameLabel};

    #[get("/hello/<name>?<caps>")]
    pub fn hello(name: &str, caps: Option<bool>, name_counter: &State<NameCounter>) -> String {
        let name = caps
            .unwrap_or_default()
            .then(|| name.to_uppercase())
            .unwrap_or_else(|| name.to_string());

        name_counter
            .get_or_create(&NameLabel { name: name.clone() })
            .inc();

        format!("Hello, {}!", name)
    }

    #[derive(Deserialize)]
    pub struct Person {
        age: u8,
    }

    #[post("/hello/<name>?<caps>", format = "json", data = "<person>")]
    pub fn hello_post(
        name: String,
        person: Json<Person>,
        caps: Option<bool>,
        name_counter: &State<NameCounter>,
    ) -> String {
        let name = caps
            .unwrap_or_default()
            .then(|| name.to_uppercase())
            .unwrap_or_else(|| name.to_string());
        name_counter
            .get_or_create(&NameLabel { name: name.clone() })
            .inc();

        format!("Hello, {} year old named {}!", person.age, name)
    }
}

#[launch]
async fn rocket() -> _ {
    let prometheus = PrometheusMetrics::new();

    let name_counter = NameCounter::default();

    {
        let mut registry = prometheus.registry().lock().await;
        registry.register("name_counter", "Count of names", name_counter.clone());
    }

    rocket::build()
        .attach(prometheus.clone())
        .manage(name_counter)
        .mount("/", routes![routes::hello, routes::hello_post])
        .mount("/metrics", prometheus)
}
