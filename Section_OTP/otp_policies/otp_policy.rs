#!/usr/bin/env rust-script
//! Set up policies to generate OTPs with an authenticator
//!
//! ```cargo
//! [dependencies]
//! clap =  { version = "3.0.14", features = ["derive"] }
//! clap-verbosity-flag = "*"
//! env_logger = "*"
//! log = "*"
//! rand = "*"
//! data-encoding = "*"
//! scone_cli = { version = "*", path="../scone_cli" }
//! shells ="*"
//! serde = "*"
//! users = "*"
//! ```

use clap::{ArgGroup, Parser};
use clap_verbosity_flag::{Verbosity, ErrorLevel};
use env_logger;
use log::{info, error};
use data_encoding::BASE32_NOPAD;
use scone_cli::{write_state,read_state,check_mrenclave,create_session,Init, random_name};
use shells::*;
use std::fs;
use std::io;
use std::io::Write;
use serde::{Deserialize, Serialize};
use users::get_current_username;

// persistent state handling

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct State {
    pub namespace: String,          // name of the namespace - name is randomly selected
    pub namespace_hash: String,     // last known hash of the namespace session
    pub session: String,            // name of session - including namespace
    pub session_hash: String,       // hash of the session
    pub session_version: u64,       // version of session that we have created last ; if different from volume_version, we need to update the session
    pub session2: String,           // name of 2nd session - this uses an OTP for access control
    pub session_hash2: String,      // hash of 2nd session
    pub mrenclave: String,          // MrEnclave of the QR code generator
    pub session_version2: u64,      // version of session that  we have created last ; if different from volume_version, we need to update the session
    pub volume_version: u64,        // triggers a key roll by increasing this version
    pub otp_image: String,          // name of the image to generate the QR code
    pub otp_binary: String,         // binary in that image that we need to execute
    pub scone_user: String,         // name of the user that executes this program
    pub scone_account : String,     // some name used in your authenticator
    pub secret: String,             // base32 encoded secret - for now in clear text. We need to protect this!
}

impl Init for State {
    fn new() -> State {
        let ns = random_name(20);
        let user = match get_current_username() {
            Some(uname) => uname.to_string_lossy().to_string(),
            None        => random_name(10),
        };

        let secret : [u8 ; 32] = rand::random();
        let state = State {
            session: format!("{}/otpqr-x", ns),
            session2: format!("{}/otpqr-reset", ns),
            namespace: ns,
            scone_user: user,
            scone_account: "SCONE OTP".to_string(),
            otp_image: "otpqr:scone".to_string(),
            otp_binary: "/bin/otpqr".to_string(),
            secret: BASE32_NOPAD.encode(&secret).into(),
            ..Default::default()
        };
        info!("Initialized state is {:?}", state);
        state
    }
}

#[derive(Parser, Debug)]
#[clap(author="Christof Fetzer", version="0.1.1", about="Create/update OTP policy.", long_about = "
This utility creates / updates a policy to manage OTPs.
")]
enum Commands {
    #[structopt(about = "Create or update OTP policies")]
    Create {
        /// Specify 'force' in case you want to update sessions even if they already exist.
        /// For example, in case you updated the session templates.
        #[clap(long)]
        force: bool,
        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },

    #[structopt(about = "Gen QR Code. Can only be executed once after a create or roll-back.")]
    GenQRCode {
        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },


    #[structopt(about = "Gen Test QR Code - this cannot be used for add authenticator!")]
    TestQRCode {
        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },

    #[structopt(about = "Add a new authenticator. This asks you for the current OTP.")]
    AddAuthenticator {
        #[clap(long)]
        ootp: Option<String>,

        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },

    #[structopt(about = "Replace OTP secret by a new one. This REMOVES the old OTP key! After that \
    you need to update your authenticator(s) using 'gen-qr-code' or 'add-authenticator'.")]
    #[clap(group(ArgGroup::new("f0rce").required(true).args(&["force"])))]
    RollForward {
        /// Must always be specified to reduce the chance of an accidental removal of the OTP secret
        #[clap(long)]
        force: bool,

        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },
}

fn init_logger(verbose : Verbosity<ErrorLevel>) {
    env_logger::Builder::new().filter_level(verbose.log_level_filter()).init();
}

fn main() {
    let cmd = Commands::parse();
    match cmd {
        Commands::Create{ force, verbose } => { init_logger(verbose); create_command(force) },
        Commands::AddAuthenticator{ ootp, verbose } => { init_logger(verbose); add_authenticator(ootp) },
        Commands::GenQRCode{ verbose } => { init_logger(verbose); gen_qr_code() },
        Commands::TestQRCode{ verbose } => { init_logger(verbose); test_qr_code() },
        Commands::RollForward{ force, verbose } => { init_logger(verbose); roll_forward(force) },
    }
}

fn roll_forward(force: bool) {

// increment version by 1!
    let mut state : State = read_state("state.js");
    state.volume_version += 1;
// create a new secret
    let secret : [u8 ; 32] = rand::random();
    state.secret = BASE32_NOPAD.encode(&secret).into();
    info!("{:?}", state);
    write_state(&state, "state.js");

// remove existing files
    let _ = fs::remove_file("single_run/once");
    let _ = fs::remove_file("single_run/volume.fspf");
    info!("Updating policies...");
    create_command(force);
}


fn gen_qr_code() {
    let state : State = read_state("state.js");

// run as docker command
    let (code, stdout, stderr) = sh!(r#"docker run --rm -w "/root" -v "$PWD:/root" -e SCONE_CAS_ADDR=scone-cas.cf -e SCONE_CONFIG_ID={}/otpqr {} {} > qr.output"#, state.session, state.otp_image, state.otp_binary);
    info!("Command: {}\nCode: {}\n{}\n", state.otp_binary, code, stdout);
    if code != 0 {
        error!("ERROR: executing 'gen-QR-code'. Code: {}\nError output:\n{}", code, stderr);
    } else {
        println!("Written QR code to file qrcode.svg.\n 1. Please 'open qrcode.svg' and scan qr code to initialize your authentication.\n 2. Remove qrcode.svg using: 'shred -n 3 -z -u qrcode.svg'\n")
    }
}


fn test_qr_code() {
    let state : State = read_state("state.js");

// run as docker command
    let (code, stdout, stderr) = sh!(r#"docker run --rm -w "/root" -v "$PWD:/root" -e SCONE_CAS_ADDR=scone-cas.cf -e SCONE_CONFIG_ID={}/test {} {} > qr.output"#, state.session, state.otp_image, state.otp_binary);
    info!("Command: {}\nCode: {}\n{}\n", state.otp_binary, code, stdout);
    if code != 0 {
        error!("ERROR: executing 'test-QR-code'. Code: {}\nError output:\n{}", code, stderr);
    } else {
        println!("Written test QR code to file test.svg.\n- This cannot be used for authorization.\n")
    }
}
fn create_command(force: bool) {
    // template for define OTP secret
    let session_template = r#"
name: {{session}}
version: "0.3"
{{predecessor_key}}: {{predecessor}}

access_policy:
    read:
    - CREATOR
    update:
    - CREATOR
    create_sessions:
    - CREATOR

services:
    - name: otpqr
      image_name: otpqr_image
## enable for release mode:
#     attestation:
#      - mrenclave:
#        - {{mrenclave}}
      environment:
        OTP_SINGLE_USE: "/root/single_run/once"
        OTP_ACCOUNT_NAME: "{{scone_account}}"
        OTP_ACCOUNT_LOGIN: "{{scone_user}}"
        OTP_SECRET: $$SCONE::otp_secret$$
        OTP_OUTPUT_FILE: "/root/qrcode.svg"
      pwd: "/root"
    - name: test
      image_name: otpqr_image
      environment:
        OTP_SINGLE_USE: "/root/single_run/test"
        OTP_ACCOUNT_NAME: "otp_test_account"
        OTP_ACCOUNT_LOGIN: "otp_test_user"
        OTP_SECRET: test
        OTP_OUTPUT_FILE: "/root/test.svg"
      pwd: "/root"

security:
    attestation:
      mode: none
    # mode: hardware
    # tolerate: [debug-mode, hyperthreading, outdated-tcb]

volumes:
    - name: single_run_{{volume_version}}
      export:
        - session: {{session2}}

images:
    - name: otpqr_image
      volumes:
        - name: single_run_{{volume_version}}
          path: /root/single_run

secrets:
    - name: otp_secret
      kind: ascii
      value: {{secret}}
      export:
        - session: {{session2}}
"#;

    // session template to add another authenticator
    // - requires OTP to be able to add the generator

    let session_template2 = r#"
name: {{session2}}
version: "0.3"
{{predecessor_key}}: {{predecessor}}

access_policy:
  read:
    - CREATOR
  update:
    - CREATOR
  create_sessions:
    - CREATOR

services:
  - name: otpqr
    image_name: otpqr_image
    command: /bin/otpqr
#    attestation:
#      - mrenclave:
#        - $MRENCLAVE
    environment:
        OTP_SINGLE_USE: "/root/single_run/once"
        OTP_ACCOUNT_NAME: "{{scone_account}}"
        OTP_ACCOUNT_LOGIN: "{{scone_user}}"
        OTP_SECRET: $$SCONE::otp_secret$$
        OTP_OUTPUT_FILE: "/root/qrcode.svg"
        OTP_RESET: "TRUE"
    pwd: "/root"

security:
  attestation:
    one_time_password_shared_secret: {{secret}}
    # one_time_password_shared_secret: $$SCONE::otp_secret:base64$$
    mode: none
    # tolerate: [debug-mode, hyperthreading, outdated-tcb]

volumes:
  - name: single_run_{{volume_version}}
    import:
      session: {{session}}
      volume: single_run_{{volume_version}}

images:
  - name: otpqr_image
    volumes:
      - name: single_run_{{volume_version}}
        path: /root/single_run

secrets:
 - name: otp_secret
   import:
     session: {{session}}
     secret: otp_secret
"#;

    let namespace_template = r#"
name: {{namespace}}
version: "0.3"
{{predecessor_key}}: {{predecessor}}

access_policy:
  read:
    - CREATOR
  update:
    - CREATOR
  create_sessions:
    - CREATOR
"#;

    // create "volume"
    let _ = fs::create_dir_all("single_run");

    let mut state : State = read_state("state.js"); // default: provide init state
    // retrieve MRENCLAVE from otp_image
    check_mrenclave(&mut state, "mrenclave", "otp_image", "otp_binary", force).expect("Failed to determine MRENCLAVE. Does image exist?"); // j, "mrenclave",
    state.namespace_hash = create_session(&state.namespace, &state.namespace_hash, namespace_template, &state, force).expect("Creating namespace");

    let force = force || state.session_version != state.volume_version;  // check if we need to update the session?
    state.session_hash = create_session(&state.session, &state.session_hash, session_template, &state, force).expect("Creating session");
    info!("Session hash = {}", state.session_hash);
    state.session_version = state.volume_version;

    let force = force || state.session_version2 != state.volume_version; // check if we need to update the session?
    state.session_hash2 = create_session(&state.session2, &state.session_hash2, session_template2, &state, force).expect("Creating session2");
    info!("Session hash2 = {}", state.session_hash);
    state.session_version2 = state.volume_version;

    write_state(&state, "state.js");
}

fn add_authenticator(ootp: Option<String>) {
    let state : State = read_state("state.js"); // default: provide init state

    let otp = if let Some(otp) = ootp {
        otp
    } else {
        let prompt = r#"
    Adding a new authenticate requires an OTP from an existing authenticator.
        - The new QR code is written to file 'qrcode.svg'
        - Starting containers can take some while. Hence, wait for a new QR code to appear on your authenticator.
    Type OTP and press enter: "#;

        print!("{}", prompt);

        // get OTP from user
        io::stdout().flush().unwrap();
        let mut otp = String::new();
        io::stdin().read_line(&mut otp).expect("Error getting OTP");
        otp.retain(|c| !c.is_whitespace());
        otp
    };

    info!("Got OTP {}", otp);
    let (code, stdout, stderr) = sh!(r#"docker run --rm -w "/root" -v "$PWD:/root" -e "SCONE_CAS_ADDR=scone-cas.cf" -e "SCONE_CONFIG_ID={}/otpqr@{}" otpqr:scone /bin/otpqr > qr.output"#, state.session2, otp);
    info!("Command: returns code: {}\n{}\n", code, stdout);
    if code != 0 {
        error!("ERROR: executing '/bin/otpqr.rs'. Code: {}\nError output:\n{}", code, stderr);
    } else {
        println!("Written QR code to file qrcode.svg.\n 1. Please 'open qrcode.svg' and scan qr code to initialize your authentication.\n 2. Remove qrcode.svg using: 'shred -n 3 -z -u qrcode.svg'\n")
    }
}
