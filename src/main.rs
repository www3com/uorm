use serde::{Deserialize, Serialize};
use std::time::Duration;
use uorm::models::db_config::UormOptions;
use uorm::pool_manager::{pool_mgr, DB};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Serialize, Deserialize, Debug)]
struct Params {
    id: i64,
    name: String,
}

#[derive(Serialize, Debug)]
struct UpdateParams<'a> {
    name: &'a str,
    id: i64,
    age: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::new("debug")).init();
    let options = UormOptions::new(
        "mysql://root:jason%40admin%402023@upbos.x3322.net:13360/upflow".into(),
    );
    let manager = pool_mgr();
    manager.register("main", &options).await?;

    tokio::task::spawn(async move {
        let p = DB.session("main").unwrap();
        let args = UpdateParams {
            name: "jason6",
            id: 1989541481361387520i64,
            age: 25,
        };
        p.execute("UPDATE app SET name=#{name} WHERE id=#{id}", &args)
            .await
            .expect("TODO: panic message");
        let rows = p
            .query::<Params, _>("SELECT * FROM app", &())
            .await
            .unwrap();
        println!("{:?}", rows);
    });
    tokio::time::sleep(Duration::from_secs(10)).await; // 休眠 3 秒
    Ok(())
}
