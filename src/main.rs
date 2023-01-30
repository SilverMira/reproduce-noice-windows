//! A basic example. Mainly for use in a test, but also shows off some basic
//! functionality.
use std::{env, error::Error};

use async_trait::async_trait;

use rmpv::Value;

use nvim_rs::{compat::tokio::Compat, create::tokio as create, Handler, Neovim, UiAttachOptions};

#[derive(Clone)]
struct NeovimHandler {}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Compat<tokio::process::ChildStdin>;

    async fn handle_request(
        &self,
        name: String,
        _args: Vec<Value>,
        _neovim: Neovim<Compat<tokio::process::ChildStdin>>,
    ) -> Result<Value, Value> {
        match name.as_ref() {
            "ping" => Ok(Value::from("pong")),
            _ => unimplemented!(),
        }
    }

    async fn handle_notify(
        &self,
        name: String,
        args: Vec<Value>,
        _neovim: Neovim<Compat<tokio::process::ChildStdin>>,
    ) {
        println!("{}: {:?}", name, args);
    }
}

#[tokio::main]
async fn main() {
    let handler: NeovimHandler = NeovimHandler {};
    let (nvim, io_handler, _) = create::new_child(handler).await.unwrap();
    let mut ui_opts = UiAttachOptions::new();
    ui_opts.set_linegrid_external(true);
    nvim.ui_attach(80, 40, &ui_opts).await.unwrap();
    println!("Attached to nvim");
    nvim.input(":").await.unwrap();

    let mut envargs = env::args();
    let _ = envargs.next();

    // Any error should probably be logged, as stderr is not visible to users.
    match io_handler.await {
        Err(joinerr) => eprintln!("Error joining IO loop: '{}'", joinerr),
        Ok(Err(err)) => {
            if !err.is_reader_error() {
                // One last try, since there wasn't an error with writing to the
                // stream
                nvim.err_writeln(&format!("Error: '{}'", err))
                    .await
                    .unwrap_or_else(|e| {
                        // We could inspect this error to see what was happening, and
                        // maybe retry, but at this point it's probably best
                        // to assume the worst and print a friendly and
                        // supportive message to our users
                        eprintln!("Well, dang... '{}'", e);
                    });
            }

            if !err.is_channel_closed() {
                // Closed channel usually means neovim quit itself, or this plugin was
                // told to quit by closing the channel, so it's not always an error
                // condition.
                eprintln!("Error: '{}'", err);

                let mut source = err.source();

                while let Some(e) = source {
                    eprintln!("Caused by: '{}'", e);
                    source = e.source();
                }
            }
        }
        Ok(Ok(())) => {}
    }
}
