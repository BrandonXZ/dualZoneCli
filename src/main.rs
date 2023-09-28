#![allow(unused)]
#![allow(non_snake_case)]
#![allow(dead_code)]
// #![allow())]

use std::error::Error;
//Internal
use std::io::{stdout, Write, ErrorKind};
use std::sync::{Arc, Mutex};
use user_input::structs::Console;

//modules
mod user_input;

//External
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::Duration;
use crossterm::{cursor, execute, queue, style, terminal, Result};
use serde::{Serialize, Deserialize};

const PROMPT: &str = "> ";

/**
 * main app starting point
 */
pub fn main() {

    let broker = init(); 
    run(broker);
}

/**
 * Simple init func to demonstrate how to implement this cli
 */
pub fn init() -> Console {
    let main_broker = Console::default();
    main_broker
}

/**
 * Async main, this is where the magic happens.
 */
#[tokio::main]
async fn run(Broker: Console) -> Result<()> {

    let mut main_inbox = Broker.rx;
    let mut user_input = Broker.tx;
    let mut stop_requested = false;

    let input_handle = tokio::spawn(async move {
        let mut reader = BufReader::new(tokio::io::stdin());
        loop {
            if stop_requested {
                break;
            }

            let mut input = String::new();
            match reader.read_line(&mut input).await {
            //user ends input
                Ok(0) | Ok(_) if input.trim().eq_ignore_ascii_case("exit") || input.trim().eq_ignore_ascii_case("quit") => {
                    user_input.send("USER_BREAK_$0uU".to_string());
                    stop_requested = true;
                    break;
                },
            //message ok
                Ok(_) => {
                    user_input.send(input.trim().to_string()).await.unwrap();
                },
            //User pressed Ctrl+C
                Err(ref e) if e.kind() == tokio::io::ErrorKind::Interrupted => {
                    // Ctrl+C was pressed
                    stop_requested = true;
                    break;
                },
            //other errors
                Err(e) => {
                    println!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
    });


    let output_handle = tokio::spawn(async move {
        let mut stdout = Arc::new(Mutex::new(stdout()));

        loop {
            let input = main_inbox.recv().await;

            let input = match input {
                Some(input) => input,
                None => { "USER_BREAK_$0uU".to_string()}
            };

            if input.clone().trim().eq_ignore_ascii_case("USER_BREAK_$0uU") {
                stop_requested = true;
                break;
            }

            queue!(
                *stdout.lock().unwrap(),
                cursor::SavePosition,
                cursor::MoveToPreviousLine(1),
                terminal::Clear(terminal::ClearType::CurrentLine),
                style::Print(input),
                cursor::RestorePosition,
                cursor::MoveToNextLine(1),
                style::Print(PROMPT),
            )
            .unwrap();
            stdout.lock().unwrap().flush().unwrap();

            // Wait for 5 seconds and clear the output
            tokio::time::sleep(Duration::from_secs(2)).await;
            queue!(
                *stdout.lock().unwrap(),
                terminal::Clear(terminal::ClearType::All),
                cursor::MoveToPreviousLine(1),
                style::Print(PROMPT),
            )
            .unwrap();
            stdout.lock().unwrap().flush().unwrap();
        }
    });

    // Wait for both tasks to complete and handle any errors
    match tokio::try_join!(input_handle, output_handle) {
        Ok(_) => {Ok(())},
        Err(e) => {
            // Handle errors from either task
            let error = std::io::Error::new(ErrorKind::UnexpectedEof, e);
            println!("Error: {:?}", error.to_string());
            Err(Box::new(error))},
    };

    Ok(())
}