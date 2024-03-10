// First thing Rust will do is look for api.rs and use that.
// If api.rs doesnt exist, it looks for ./api/mod.rs
mod api;
mod repository;
mod model;

use api::task::{
    get_task, 
    submit_task, 
    //fail_task,
    start_task,
    complete_task,
    //pause_task,
};

use actix_web::{HttpServer, App, web::Data, middleware::Logger};
use repository::ddb::DDBRepository;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Logging variables that actix web reads to determine if logging enabled. 
    std::env::set_var("RUST_LOG", "debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // AWS Config
    //https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-envvars.html
    
    let config = aws_config::load_from_env().await;
    /*
    Because this closure where we are contructing the app arent async, 
    we cant instantiate the AWS config from within it.
    But we also cannot instantiate the DDBRepository from outside the closure
    as it is not thread safe. 
    */

    // Creating HTTP server struct
    // Anything closure references from before the closure
    // we want to move ownership to new closure hence "Move"
    HttpServer::new(move || { 
        // The following two vars are not deamed by the thread compiler as thread safe:
        let ddb_repo: DDBRepository = DDBRepository::init(
            String::from("task"),
            config.clone(),
        );
        let ddb_data = Data::new(ddb_repo);
        // setup default logger object
        let logger = Logger::default();
        App::new()
        .wrap(logger) // Allows logging
        .app_data(ddb_data)
        .service(get_task) // Specifies handler functions
        .service(submit_task)
        //.service(fail_task)
        //.service(pause_task)
        .service(start_task)
        .service(complete_task)
    })
    .bind(("127.0.0.1", 8000))? // This Can cause an error so propagate error up the stack.
    .run()
    .await
}
