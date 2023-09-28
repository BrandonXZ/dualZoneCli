#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused)]

//Internal
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc};

//External
use derive_getters::Getters;
use serde::{Serialize, Deserialize};
use tokio::sync::{mpsc::{Sender, Receiver, channel}, Mutex};
use tokio::io::stdout;

/**
 * The struct defining the Console Object, which is designed to be a broker for all other modules to communicate with the main thread and output to the console without collisions
 *  ~ The Console struct is designed to be a singleton, and is therefore not clonable.
 *  ~ init() creates the receiver which is not clonable. 
 *  ~ new_sender() creates clones of the sender, which can be passed to other modules.
 *  ~ Each Sender must be in the Authorized list and NOT in the BlackListed list to be able to send messages to the console or communicate with the main thread.
 *  ~ Senders and receivers are currently only enabled to handle strings, but this will be updated to handle a more complex data structures in the future.
 *  ~ Modules that need their own Console Broker should call SubConsole to have their output configured to the module workspace via rabbitMQ(WIP).
 * !  Console `may` be called multiple times if called from a different thread when a failover occurs, see major failover documentation.
 */

#[derive(Debug, Getters)]
pub struct Console<> {
    pub tx: Sender<String>,
    pub rx: Receiver<String>,
    pub stdout: Arc<Mutex<tokio::io::Stdout>>,
    pub phonebook: HashMap<String, (String, SenderStatus)>,
    pub Authorized: HashMap<String, Sender<String>>,
    pub BlackListed: HashMap<String, Sender<String>>
}

/**
 * Default impl for the Console struct mirrors the init() function with one exception, it returns the Console struct instead of the Sender, Receiver tuple.
 */
impl Default for Console {
    fn default() -> Self {
        //call init to create the default impl
        let (tx, rx): (Sender<String>, Receiver<String>) = channel(100);
        let stdout = Arc::new(Mutex::new(stdout()));
        let Authorized: HashMap<String, Sender<String>> = HashMap::new();
        let phonebook: HashMap<String, (String, SenderStatus)> = HashMap::new();
        let BlackListed: HashMap<String, Sender<String>> = HashMap::new();
        let console = Console {
            tx,
            rx,
            stdout,
            phonebook,
            Authorized,
            BlackListed,
        };
        console
    }
}

impl Console<> {
    /**
     * Start the Console Broker and return a Sender<String> to the caller.
     */
    pub fn init() -> (Sender<String>, Receiver<String>) {
        let (tx, rx): (Sender<String>, Receiver<String>) = channel(100);
        let stdout = Arc::new(Mutex::new(stdout()));
        let Authorized: HashMap<String, Sender<String>> = HashMap::new();
        let phonebook: HashMap<String, (String, SenderStatus)> = HashMap::new();
        let BlackListed: HashMap<String, Sender<String>> = HashMap::new();
        let console = Console {
            tx,
            rx,
            stdout,
            phonebook,
            Authorized,
            BlackListed,
        };
        (console.tx.clone(), console.rx)
    }

    /**
     * Create a new Sender, add it to the Authorized list and return it.
     */
    pub fn new_sender(&mut self, name:String) -> Sender<String> {
        let sender = self.tx.clone();
        let signed_name = Console::generate_id(name.clone());
        self.Authorized.insert(signed_name.clone(), sender.clone());
        self.phonebook.insert(signed_name, (name, SenderStatus::Authorized));
        sender
    }

    /**
     * Added security for the identifiers
     */
    fn generate_id(identifier: String) -> String {
        let mut hasher = DefaultHasher::new();
        identifier.hash(&mut hasher);
        let signed_identifier = hasher.finish();
        signed_identifier.to_string()
    }

    /**
     * Get the Plaintext name fom the generated id
     */
    fn get_plaintext_name(&self, search_name: String) -> String {
        
        let name = self.phonebook.get(&search_name);
        match name {
            Some(n) => {
                n.0.to_string()
            },
            None => {
                "Name not found".to_string()
            }
        }
    }

    /**
     * Get the SenderStatus from the generated id
     */
    fn get_sender_status(&self, search_name: String) -> SenderStatus {
        let status = self.phonebook.get(&search_name);
        match status {
            Some(s) => {
                s.1.clone()
            },
            None => {
                SenderStatus::NotInPhonebook
            }
        }
    }

    /**
     * Get the SenderStatus from the plaintext name
     */
    fn get_sender_status_by_name(&self, search_name: String) -> SenderStatus {
        let id = Console::generate_id(search_name);
        let status = self.phonebook.get(&id);
        match status {
            Some(s) => {
                s.1.clone()
            },
            None => {
                SenderStatus::NotInPhonebook
            }
        }
    }

    /**
     * Update the Senderstatus by either identifier or plaintext name
     */
    fn change_sender_status(&mut self, search_name: String, new_status: SenderStatus) {
        let id = Console::generate_id(search_name.clone());
        let status = self.phonebook.get(&id);
        match status {
            Some(s) => {
                self.phonebook.insert(id, (s.0.clone(), new_status));
            },
            None => {
                //check the phonebook for the search_name directly
                let status = self.phonebook.get(&search_name);
                match status {
                    Some(s) => {
                        self.phonebook.insert(id, (s.0.clone(), new_status));
                    },
                    None => {
                        println!("Name not found using both plaintext and id");
                    }
                }
                println!("Name not found using id");
            }
        }
    }

        /**
     * Adds a sender to the blacklist
     */
    pub fn add_to_blacklist(&mut self, identifier: String) {
        let id = Console::generate_id(identifier);
        let sender = self.Authorized.remove(&id);
        match sender {
            Some(s) => {
                self.BlackListed.insert(id, s);
            },
            None => {
                println!("Sender not found");
            }
        }
    }

    /**
     * Get just the names of everyone on the blacklist
     */
    pub fn get_blacklist_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for (name, _) in self.BlackListed.iter() {
            names.push(name.to_string());
        }
        names
    }

    /**
     * show all names in the blacklist in plaintext
     */
    pub fn show_blacklist(&self) {
        println!("Blacklisted Names: ");
        for (name, _) in self.BlackListed.iter() {
            println!("{}", name);
        }
    }

    /**
     * Get just the names of everyone on the authorized list
     */
    pub fn get_authorized_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for (name, _) in self.Authorized.iter() {
            names.push(name.to_string());
        }
        names
    }
    /**
     * show all names in the authorized list in plaintext
     */
    pub fn show_authorized(&self) {
        println!("Authorized Names: ");
        for (name, _) in self.Authorized.iter() {
            println!("{}", name);
        }
    }
}

#[derive(Debug, Clone)]
pub enum SenderStatus {
    Authorized,
    BlackListed,
    NotAuthorized,
    NotInPhonebook,
}

#[derive(Debug, Clone)]
pub enum PriorityStatus {
    Urgent,
    Critical,
    Notice,
    Warning,
    Exception,
    Delay,
    Verbose, 
    Normal, 
    Ignore, 
    Informational
}

/******************************************************************************************************************************************************************************/
/**
 * ! WIP
 * This Struct is to essentially Mirror what the Console struct does, but for other modules to use.
 * Instead of the shared output being used to communicate with the main thread, it will be used to communicate with the module's thread.
 * The idea is to have this set up to implement RabbitMQ for easier system distribution.
 * Each module that calls a subConsole should have a workspace loop, that is to say, its using the established async runtime to run its own loop independent of other processes.
 * This really shouldnt be used for synchronous modules as it could block and will likely cause a deadlock.
 */
#[derive(Debug, Getters)]
struct SubConsole<> {
    tx: Sender<String>,
    rx: Receiver<String>,
    // stdout: Arc<Mutex<tokio::io::Stdout>>, //Rabbit MQ will handle this
    phonebook: HashMap<String, (String, SenderStatus)>,
    Authorized: HashMap<String, Sender<String>>,
    BlackListed: HashMap<String, Sender<String>>
}

/*******************************************************************************Processes**************************************************************************************/