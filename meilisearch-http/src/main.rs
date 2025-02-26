use std::env;

use actix_web::HttpServer;
use meilisearch_http::{create_app, setup_meilisearch, Opt};
use meilisearch_lib::MeiliSearch;
use structopt::StructOpt;

#[cfg(all(not(debug_assertions), feature = "analytics"))]
use meilisearch_http::analytics;

#[cfg(target_os = "linux")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

/// does all the setup before meilisearch is launched
fn setup(opt: &Opt) -> anyhow::Result<()> {
    let mut log_builder = env_logger::Builder::new();
    log_builder.parse_filters(&opt.log_level);
    if opt.log_level == "info" {
        // if we are in info we only allow the warn log_level for milli
        log_builder.filter_module("milli", log::LevelFilter::Warn);
    }

    log_builder.init();

    Ok(())
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    setup(&opt)?;

    match opt.env.as_ref() {
        "production" => {
            if opt.master_key.is_none() {
                anyhow::bail!(
                    "In production mode, the environment variable MEILI_MASTER_KEY is mandatory"
                )
            }
        }
        "development" => (),
        _ => unreachable!(),
    }

    let meilisearch = setup_meilisearch(&opt)?;

    // Setup the temp directory to be in the db folder. This is important, since temporary file
    // don't support to be persisted accross filesystem boundaries.
    meilisearch_http::setup_temp_dir(&opt.db_path)?;

    #[cfg(all(not(debug_assertions), feature = "analytics"))]
    if !opt.no_analytics {
        let analytics_data = meilisearch.clone();
        let analytics_opt = opt.clone();
        tokio::task::spawn(analytics::analytics_sender(analytics_data, analytics_opt));
    }

    print_launch_resume(&opt);

    run_http(meilisearch, opt).await?;

    Ok(())
}

async fn run_http(data: MeiliSearch, opt: Opt) -> anyhow::Result<()> {
    let _enable_dashboard = &opt.env == "development";
    let opt_clone = opt.clone();
    let http_server = HttpServer::new(move || create_app!(data, _enable_dashboard, opt_clone))
        // Disable signals allows the server to terminate immediately when a user enter CTRL-C
        .disable_signals();

    if let Some(config) = opt.get_ssl_config()? {
        http_server
            .bind_rustls(opt.http_addr, config)?
            .run()
            .await?;
    } else {
        http_server.bind(&opt.http_addr)?.run().await?;
    }
    Ok(())
}

pub fn print_launch_resume(opt: &Opt) {
    let commit_sha = option_env!("VERGEN_GIT_SHA").unwrap_or("unknown");
    let commit_date = option_env!("VERGEN_GIT_COMMIT_TIMESTAMP").unwrap_or("unknown");

    let ascii_name = r#"
888b     d888          d8b 888 d8b  .d8888b.                                    888
8888b   d8888          Y8P 888 Y8P d88P  Y88b                                   888
88888b.d88888              888     Y88b.                                        888
888Y88888P888  .d88b.  888 888 888  "Y888b.    .d88b.   8888b.  888d888 .d8888b 88888b.
888 Y888P 888 d8P  Y8b 888 888 888     "Y88b. d8P  Y8b     "88b 888P"  d88P"    888 "88b
888  Y8P  888 88888888 888 888 888       "888 88888888 .d888888 888    888      888  888
888   "   888 Y8b.     888 888 888 Y88b  d88P Y8b.     888  888 888    Y88b.    888  888
888       888  "Y8888  888 888 888  "Y8888P"   "Y8888  "Y888888 888     "Y8888P 888  888
"#;

    eprintln!("{}", ascii_name);

    eprintln!("Database path:\t\t{:?}", opt.db_path);
    eprintln!("Server listening on:\t\"http://{}\"", opt.http_addr);
    eprintln!("Environment:\t\t{:?}", opt.env);
    eprintln!("Commit SHA:\t\t{:?}", commit_sha.to_string());
    eprintln!("Commit date:\t\t{:?}", commit_date.to_string());
    eprintln!(
        "Package version:\t{:?}",
        env!("CARGO_PKG_VERSION").to_string()
    );

    #[cfg(all(not(debug_assertions), feature = "analytics"))]
    {
        if opt.no_analytics {
            eprintln!("Anonymous telemetry:\t\"Disabled\"");
        } else {
            eprintln!(
                "
Thank you for using MeiliSearch!

We collect anonymized analytics to improve our product and your experience. To learn more, including how to turn off analytics, visit our dedicated documentation page: https://docs.meilisearch.com/learn/what_is_meilisearch/telemetry.html

Anonymous telemetry:   \"Enabled\""
            );
        }
    }

    eprintln!();

    if opt.master_key.is_some() {
        eprintln!("A Master Key has been set. Requests to MeiliSearch won't be authorized unless you provide an authentication key.");
    } else {
        eprintln!("No master key found; The server will accept unidentified requests. \
            If you need some protection in development mode, please export a key: export MEILI_MASTER_KEY=xxx");
    }

    eprintln!();
    eprintln!("Documentation:\t\thttps://docs.meilisearch.com");
    eprintln!("Source code:\t\thttps://github.com/meilisearch/meilisearch");
    eprintln!("Contact:\t\thttps://docs.meilisearch.com/resources/contact.html or bonjour@meilisearch.com");
    eprintln!();
}
