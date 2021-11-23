use std::net::{TcpStream};
use std::io::{self, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use rsa::pkcs8::FromPrivateKey;
use sha2::Digest;
use std::str::from_utf8;
use rsa::{PublicKey, RsaPublicKey, RsaPrivateKey, PaddingScheme};

const PRIVATE_KEY: &'static str = include_str!("../../private_key.pem");
const SERVER: &'static str = "localhost:5555";

fn user_verify_nonaction(ty: &[u8; 3], msg: &str) -> Result<[u8; 53], io::Error> {
    println!("Please verify the request: \n > {:50}", msg);
    print!("Is this correct? [y/N]: ");
    let mut approved = String::new();
    std::io::stdout().flush().unwrap();
    io::stdin().read_line(&mut approved)?;
    let approved = approved.trim();

    if approved.to_uppercase() == "Y" {
        let mut output: [u8; 53] = [0; 53];
        let (otyp, request) = output.split_at_mut(3);
        otyp.copy_from_slice(ty);
        request.copy_from_slice(msg.as_bytes());
        Ok(output)
    } else {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
}

fn user_verify_action(ty: &[u8; 3], msg: &str) -> Result<[u8; 311], io::Error> {
    fn sign_action(ty: &[u8; 3], msg: &str) -> [u8; 311] {
        let mut signed_msg: [u8; 311] = [0_u8; 311];
        let (otyp, inner) = signed_msg.split_at_mut(3);
        otyp.copy_from_slice(ty);
        let (dat, sig) = inner.split_at_mut(52);

        // Sign
        let sk = RsaPrivateKey::from_pkcs8_pem(PRIVATE_KEY).expect("failed to get private key");
        let pk = RsaPublicKey::from(&sk);

        let timestamped_msg = [&timestamp()[..], msg.as_bytes()].concat();

        let hash: &[u8] = &sha2::Sha512::digest(&timestamped_msg[..])[..];
        let padding = PaddingScheme::new_pkcs1v15_sign(Some(rsa::Hash::SHA3_512));
        let signature = sk.sign(padding, &hash).expect("failed to sign");
        
        // Check Signature
        let padding = PaddingScheme::new_pkcs1v15_sign(Some(rsa::Hash::SHA3_512));
        pk.verify(padding, &hash, &signature)
            .expect("Signature Not Valid");

        // println!("{}", unsafe {std::str::from_utf8_unchecked(&signature[..])}); // unique id gen
        
        // copy slices into output
        dat.copy_from_slice(&timestamped_msg[..]);
        sig.copy_from_slice(&signature);

        signed_msg
    }

    println!("Please confirm the ACTION: \n > {:44}", msg); // TODO: checksize
    print!("Is this correct? [y/N]: ");
    let mut approved = String::new();
    std::io::stdout().flush().unwrap();
    io::stdin().read_line(&mut approved)?;
    let approved = approved.trim();

    if approved.to_uppercase() == "Y" {
        Ok(sign_action(ty, &msg))
    } else {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
}

// Time the best ID system xD
fn timestamp() -> [u8; 8] {
    let time = SystemTime::now();
    let seconds_since_epoch = time.duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch").as_secs();
    seconds_since_epoch.to_be_bytes()
}

fn main() -> Result<(), io::Error> {
    while {
        print!("Request Type: ");
        let mut request_type = String::new(); 
        std::io::stdout().flush().unwrap();
        io::stdin().read_line(&mut &mut request_type)?;
        request_type = request_type.to_uppercase();
        
        match request_type.as_str().trim() {
            "SEN" => {
                print!("Uname: ");
                let mut uname = String::new(); 
                std::io::stdout().flush().unwrap();
                io::stdin().read_line(&mut uname)?;
            
                print!("To: ");
                let mut dest = String::new(); 
                std::io::stdout().flush().unwrap();
                io::stdin().read_line(&mut dest)?;
                
                print!("amount: ");
                let mut amount = String::new(); 
                std::io::stdout().flush().unwrap();
                io::stdin().read_line(&mut amount)?;

                let transaction = format!("{:44}", format!("{} {} {}", uname.trim(), dest.trim(), amount.trim()));    
                let verification = user_verify_action(b"SEN", &transaction);

                if let Ok(msg) = verification {
                    make_request(&msg, false);
                } else {
                    println!("Transaction NOT Sent.");
                }
            }
            "BAL" => {
                print!("Uname: ");
                let mut uname = String::new(); 
                std::io::stdout().flush().unwrap();
                io::stdin().read_line(&mut uname)?;

                let request = format!("{:50}", uname.trim());
                let verification = user_verify_nonaction(b"BAL", &request);

                if let Ok(msg) = verification {
                    make_request(&msg, true);
                } else {
                    println!("Balance request NOT sent.");
                }
            },
            "OWE" => {
                print!("<Uname> Owes (* for everyone): ");
                let mut owes = String::new(); 
                std::io::stdout().flush().unwrap();
                io::stdin().read_line(&mut owes)?;
            
                print!("To <Uname> (* for everyone): ");
                let mut to = String::new(); 
                std::io::stdout().flush().unwrap();
                io::stdin().read_line(&mut to)?;

                let request = format!("{:50}", format!("{} {}", owes.trim(), to.trim()));
                let verification = user_verify_nonaction(b"OWE", &request);

                if let Ok(msg) = verification {
                    make_request(&msg, true);
                } else {
                    println!("Debt request NOT sent.");
                }
            },
            other => {
                eprintln!("{} is not a valid request.", other);
            }
        }

        // Another Request?
        print!("Would You like to make another request? [y/N]: ");
        let mut approved = String::new();
        std::io::stdout().flush().unwrap();
        io::stdin().read_line(&mut approved)?;
        let approved = approved.trim();
    
        approved.to_uppercase() == "Y"
    } {}

    Ok(())
}

/// Make request from server.
fn make_request(msg: &[u8], await_data: bool) {
    match TcpStream::connect(SERVER) {
        Ok(mut stream) => {
            println!("Successfully connected");

            stream.write(msg).unwrap();

            println!("Sent request, awaiting confirmation...");

            let mut confirmation = [0_u8; 3];
            match stream.read_exact(&mut confirmation) {
                Ok(_) => {
                    match (&confirmation, await_data) {
                        (b"OK ", true) => {
                            println!("Received Confirmation, awaiting data...");
                            let mut data = [0_u8; 8];
                            match stream.read_exact(&mut data) {
                                Ok(_) => {
                                    println!("Answer: {}", i64::from_be_bytes(data));
                                },
                                Err(e) => {
                                    println!("Failed to receive data: {}", e);
                                }
                            }
                        },
                        (b"OK ", false) => {
                            println!("Confirmation Received. Success");
                        },

                        // Errors
                        (b"E00", _) => { println!("Error 00: Bad send. This should be unreachable."); },
                        (b"E01", _) => { println!("Error 01: Bad timestamp. Your transaction took too long or was sent within a second of your last transaction."); },
                        (b"E02", _) => { println!("Error 02: Your user is not registered with the server. Please create an account with the signup binary."); }, 
                        (b"E03", _) => { println!("Error 03: Rejected badly signed transaction."); },
                        // E04 is an error for the signup binary :: TODO remap error codes
                        (b"E05", _) => { println!("Error 05: The user you tried to send to does not exist."); },

                        (other, _) => {
                            let text = from_utf8(other).unwrap();
                            println!("Unexpected reply: {}", text);
                        },
                    }
                },
                Err(e) => {
                    println!("Failed to receive confirmation: {}", e);
                }
            }
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
}
