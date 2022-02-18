use google_authenticator::{GoogleAuthenticator,ErrorCorrectionLevel};
use std::env;
use std::process;
use std::fs::File;
use colored::*;
use std::io::Write;

fn get_env(variable : &str) -> Result<String, String> {
    match env::var(&variable) {
        Ok(value) => Ok(value),
        Err(_) => Err(format!("environment variable {} not defined", &variable)),
    }
}

fn print_qr() -> Result<(), String> {
    let file_name = get_env("OTP_SINGLE_USE")?;
    let account_name = get_env("OTP_ACCOUNT_NAME")?;
    let account_login = get_env("OTP_ACCOUNT_LOGIN")?;
    let secret = get_env("OTP_SECRET")?;
    let filename = get_env("OTP_OUTPUT_FILE")?;
    // if no reset, we need to check if file exists
    if let Err(_) = env::var("OTP_RESET") {
        match File::open(&file_name) {
            Err(_why) => (),
            Ok(_) => return Err(format!("OTP_SINGLE_USE (={}) already exists!", file_name)),
        }
    };
  
    let file = match File::create(&file_name) {
        Ok(file) => file,
        Err(why) => return Err(format!("Failed to create file {}: Error {}", &file_name, why)),
    };
    match file.sync_all() {
        Ok(_) => (),
        Err(why) => return Err(format!("Failed to sync file {}: Error {}", &file_name, why)),
    }
    let auth = GoogleAuthenticator::new();
    
    match env::var("OTP_URL") {
        Ok(_val) => {
            let mut w = File::create(filename).unwrap();
            writeln!(&mut w, "open {}", auth.qr_code_url(&secret, &account_login, &account_name, 200, 200, ErrorCorrectionLevel::High)).unwrap();
        }
        Err(_e) => {
            let mut w = File::create(filename).unwrap();
            writeln!(&mut w, "{}", auth.qr_code(&secret, &account_login, &account_name, 200, 200, ErrorCorrectionLevel::High).unwrap()).unwrap();
        }
    };
    Ok(())
}

fn usage() {
    let usage = r#"
otpqr: generates QR code for an authenticator app. Single use only.
       This is always used via a policy and hence, convenience is not important.
       We use therefore environment variables to define all parameters.
 - You need to define the following required environment variables:
   - OTP_ACCOUNT_NAME:  the account name associated with this OTP code
   - OTP_ACCOUNT_LOGIN: the user name associated with this OTP code
   - OTP_SECRET:        the secret used for generating the OTPs
   - OTP_SINGLE_USE:    the file to track single use
   - OTP_OUTPUT_FILE:   the output file for URL or SVG
 - You may define the following environment variable:
   - OTP_URL:           if set, print URL - else SVG for QR code
"#;
    eprintln!("{}", usage);
}

fn main() {
    if let Err(msg) = print_qr() {
        usage();
        eprintln!("{}:  {}", "error: opt_qr".red(), msg.magenta());
        process::exit(0x01)
    }
}
