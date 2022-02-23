#!/usr/bin/env rust-script
//!
//! ```cargo
//! [package]
//! name = "print-otp"
//! version = "0.1.0"
//! edition = "2021"
//!
//! [dependencies]
//! google-authenticator = { version = "0.3.0", features = ["with-qrcode"], default-features = false }
//! ```
use std::env;
use google_authenticator::{GoogleAuthenticator};

fn main() {
    let args: Vec<String> = env::args().collect();
    // assumes that first argument is the OTP secret - otherwise, it might panic!
    print!("{}",GoogleAuthenticator::new().get_code(&args[1],0).unwrap().as_str());
}