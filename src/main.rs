use std::collections::VecDeque;
use std::convert::Infallible;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use db::database::{Database, Save};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use uuid::Uuid;
use db::image::Image;
use db::semantic_vector::SemanticVec;

struct Address {
    ip: String,
    port: u16
}

impl Address {
    fn new(port: u16, ip: String) -> Self {
        Address {
            ip,
            port
        }
    }

    async fn listener(&self) -> Result<TcpListener, Box<dyn Error>> {
        match TcpListener::bind(format!("{}:{}", self.ip, self.port)).await {
            Ok(listener) => Ok(listener),
            Err(e) => {
                Err(Box::new(e))
            }
        }
    }

    async fn stream(&self) -> TcpStream {
        TcpStream::connect(format!("{}:{}", self.ip, self.port)).await.unwrap()
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
enum Command {
    Add (String, Vec<f32>),
    FromSentence (String),
    IsExist (String),
}

#[derive(Serialize, Deserialize)]
enum CommandSend{
    AddToDb(String)
}
#[tokio::main]
async fn main() {
    let mut db = Database::new().unwrap();

    let _task_list: VecDeque<Command> = VecDeque::new();
    let task_list = Arc::new(Mutex::new(_task_list));

    // start server
    let address = Address::new(7879, "127.0.0.1".to_string());
    let address_em = Address::new(7878, "127.0.0.1".to_string());

    let listener = address.listener().await.unwrap();
    let mut buffer = [0; 2048];

    tokio::spawn({
        let mut task_list = task_list.clone();
        async move{
            loop {
                let mut task_list_clone = task_list.lock().await;
                if !task_list_clone.is_empty() {
                    let command = task_list_clone.pop_front().unwrap();
                    std::mem::drop(task_list_clone);
                    match command {
                        Command::Add(path, semantic_vectors) => {
                            //TODO: DO I NEED TITLE?????.
                            println!("Adding to database");

                            let mut image = Image::new(path, "hello".to_string());
                            let semantic_vector = SemanticVec::from_vec(semantic_vectors);

                            image.set_semantic_vector(semantic_vector);

                            if let Some(connection) = &mut db.connection {
                                match image.save(connection) {
                                    Ok(_) => { println!("Image saved successfully") },
                                    Err(_) => { println!("Error while adding an image!!!") }
                                }
                            } else {
                                println!("Error while opening database");
                                continue;
                            }
                        },
                        Command::FromSentence(sentence) => {

                        },
                        Command::IsExist(path ) => {
                            match db.exists_image_by_path(path.as_str()) {
                                Ok(exist) => {
                                    if !exist {
                                        let command = CommandSend::AddToDb(path);
                                        let command = to_string(&command).unwrap();
                                        let command = command.as_bytes();

                                        let mut stream = address_em.stream().await;
                                        stream.writable().await.unwrap();
                                        println!("writable");
                                        stream.write_all(command).await.unwrap();
                                        println!("command has been send");
                                    }else {
                                        println!("Image already exist in db");
                                    }
                                }
                                Err(_) => {
                                    //TODO error
                                    println!("Error while looking for image!!!");
                                }
                            }
                        }
                        _ => {
                            println!("Not the right service");
                        }
                    }
                    //println!("Doing command {:?}", command);
                } else {
                    println!("There are no tasks aviable at the moment");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }
    });

    loop {
        println!("wating for data");
        let (stream, _) =  listener.accept().await.unwrap();
        stream.readable().await.unwrap();
        println!("Connection started");
        match stream.try_read(&mut buffer) {
            Ok(0) => {
                println!("There is no data to read!!!");
                break;
            },
            Ok(buffer_size) => {

                println!("{}", String::from_utf8(buffer[..buffer_size].to_vec()).unwrap());
                let command: Command = serde_json::from_slice(&buffer[..buffer_size].to_vec()).unwrap();

                let mut task_list_clone = task_list.clone();
                let mut task_list_clone = task_list_clone.lock().await;
                task_list_clone.push_back(command);
                println!("Added");
            },
            Err(_) => {
                println!("Error while reading!!!!");
                break;
            }
        }
    }
    println!("Crash");

}
