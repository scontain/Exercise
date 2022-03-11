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
//! scone_cli = { version = "*", path="../../Section_OTP/scone_cli" }
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
use std::fs::{read_to_string, write, remove_file, create_dir_all};
use std::io;
use std::io::Write;
use std::path::Path;
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
    pub session_version2: u64,      // version of session that  we have created last ; if different from volume_version, we need to update the session

    pub mrenclave: String,          // MrEnclave of the QR code generator
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
            session: format!("{}/cosign", ns),
            session2: format!("{}/cosign-reset", ns), // Needed?
            namespace: ns,
            scone_user: user,
            scone_account: "SCONE cosign".to_string(),
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
    #[structopt(about = "Create or update a policy for cosign in a separate namespace")]
    Create {
        /// Prefix of the file that contains the policies. We add a number and suffix .yml.
        /// The default files are policy1.yml and policy2.yml
        #[clap(long, default_value="policy")]
        prefix: String,

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
        otp: Option<String>,

        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },


    #[structopt(about = "Generate a new key pair. This asks you for the current OTP.")]
    GenKeypair {
        #[clap(long)]
        otp: Option<String>,

        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },


    #[structopt(about = "Sign an image. This asks you for the current OTP unless you specify --otp")]
    SignImage {
        #[clap(long)]
        otp: Option<String>,

        #[clap(long)]
        image: String,

        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },


    #[structopt(about = "Verify an image. This asks you for the current OTP unless you specify --otp")]
    VerifyImage {
        #[clap(long)]
        otp: Option<String>,

        #[clap(long)]
        image: String,

        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },

    #[structopt(about = "Replace OTP secret by a new one. This REMOVES the old OTP key! After that \
    you need to update your authenticator(s) using 'gen-qr-code' or 'add-authenticator'.")]
    #[clap(group(ArgGroup::new("f0rce").required(true).args(&["force"])))]
    RollForward {
        /// Prefix of the file that contains the policies. We add a number and suffix .yml.
        /// The default files are policy1.yml and policy2.yml
        #[clap(long, default_value="policy")]
        prefix: String,

        /// Must always be specified to reduce the chance of an accidental removal of the OTP secret
        #[clap(long)]
        force: bool,

        #[clap(flatten)]
        verbose: Verbosity<ErrorLevel>,
    },


    #[structopt(about = "Generate default policies. These policies can be customized before creating updating the policy.")]
    GenPolicies {
        /// Prefix of the file that contains the policies. We add a number and suffix .yml.
        /// The default files are policy_namespace.yml, policy_remote.yml and policy_admin.yml
        #[clap(long, default_value="policy")]
        prefix: String,

        /// The writing fails if the policy already exists. Use force to overwrite the existing policy.
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
        Commands::Create{ prefix, force, verbose } => { init_logger(verbose); create_command(&prefix, force) },
        Commands::AddAuthenticator{ otp, verbose } => { init_logger(verbose); add_authenticator(otp) },
        Commands::GenQRCode{ verbose } => { init_logger(verbose); gen_qr_code() },
        Commands::TestQRCode{ verbose } => { init_logger(verbose); test_qr_code() },
        Commands::RollForward{ prefix, force, verbose } => { init_logger(verbose); roll_forward(&prefix, force) },
        Commands::GenPolicies{ prefix, force, verbose } => { init_logger(verbose); write_policies(&prefix, force) },
        Commands::GenKeypair{ otp, verbose } => { init_logger(verbose); gen_keypair(otp) },
        Commands::SignImage{ otp, image, verbose } => { init_logger(verbose); sign_image(otp, image) },
        Commands::VerifyImage{ otp, image, verbose } => { init_logger(verbose); verify_image(otp, image) },
    }
}

fn write_policies(prefix : &str, force : bool) {
    let filename = format!("{}_namespace.yml", prefix);
    write_file(&filename, force, SESSION_TEMPLATE0);
    let filename = format!("{}_admin.yml", prefix);
    write_file(&filename, force, SESSION_TEMPLATE1);
    let filename = format!("{}_remote.yml", prefix);
    write_file(&filename, force, SESSION_TEMPLATE2);
}

fn write_file(filename: &str, force: bool, content: &str) {
    if force || !Path::new(filename).exists() {
        write(filename, content).unwrap_or_else( | _ | panic !("Unable to write file '{}'", filename));
        info!("Written policy to file {}.", filename);
    } else {
        error!("File {} already exists. Use --force to overwrite.", filename)
    }
}

fn read_policies(prefix : &str) ->  (String, String, String) {
    let namespace = format!("{}_namespace.yml", prefix);
    let admin = format!("{}_admin.yml", prefix);
    let remote = format!("{}_remote.yml", prefix);
    (read_to_string(&namespace).expect("namespace policy"),
     read_to_string(&admin).expect("admin policy"),
     read_to_string(&remote).expect("admin policy"))
}


static SESSION_TEMPLATE0 : &str = r##"#
# Simple Namespace Template
# - Only creator has access to this namespace
#
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
"##;

static SESSION_TEMPLATE1 : &str = r#"
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

static SESSION_TEMPLATE2 : &str = r#"
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
    - name: generate-key-pair
      command: cosign generate-key-pair
      image_name: cosign_image
## enable for release mode:
#     attestation:
#      - mrenclave:
#        - {{mrenclave}}
      environment:
        COSIGN_PASSWORD: $$SCONE::cosign_password$$
      pwd: "/root/cosign_keys"
    - name: sign
      command: cosign sign --key /root/cosign_keys/cosign.key @@1
      image_name: cosign_image
## enable for release mode:
#     attestation:
#      - mrenclave:
#        - {{mrenclave}}
      environment:
        COSIGN_PASSWORD: $$SCONE::cosign_password$$
        COSIGN_DOCKER_MEDIA_TYPES: 1
        GOFLAGS:"-buildmode: pie"
        HOSTNAME: 3e458b1a100f
        SCONE_ALLOW_DLOPEN: 0
        SCONE_HEAP: 1G
        PWD: /root
        SCONE_SYSLIBS: 1
        HOME: /root
        GOLANG_VERSION: 1.17.6
        TERM: xterm
        SHLVL: 1
        PATH: /go/bin:/usr/local/go/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
        GOPATH: /go
      pwd: "/root"
    - name: verify
      command: cosign verify --key /root/cosign_keys/cosign.pub @@1
      image_name: cosign_image
## enable for release mode:
#     attestation:
#      - mrenclave:
#        - {{mrenclave}}
      environment:
        COSIGN_PASSWORD: $$SCONE::cosign_password$$
        COSIGN_DOCKER_MEDIA_TYPES: 1
        GOFLAGS: "-buildmode: pie"
        HOSTNAME: 3e458b1a100f
        SCONE_ALLOW_DLOPEN: 0
        SCONE_HEAP: 1G
        PWD: /root
        SCONE_SYSLIBS: 1
        HOME: /root
        GOLANG_VERSION: 1.17.6
        TERM: xterm
        SHLVL: 1
        PATH: /go/bin:/usr/local/go/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
        GOPATH: /go
      pwd: "/root"
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
  - name: cosign_volume
  - name: single_run_{{volume_version}}
    import:
      session: {{session}}
      volume: single_run_{{volume_version}}

images:
  - name: cosign_image
    volumes:
      - name: cosign_volume
        path: /root/cosign_keys
  - name: otpqr_image
    volumes:
      - name: single_run_{{volume_version}}
        path: /root/single_run


secrets:
 - name: otp_secret
   import:
     session: {{session}}
     secret: otp_secret
 - name: cosign_password
   kind: ascii
   size: 32
"#;


fn roll_forward(prefix : &String, force: bool) {

// increment version by 1!
    let mut state : State = read_state("state.js");
    state.volume_version += 1;
// create a new secret
    let secret : [u8 ; 32] = rand::random();
    state.secret = BASE32_NOPAD.encode(&secret).into();
    info!("{:?}", state);
    write_state(&state, "state.js");

// remove existing files
    let _ = remove_file("single_run/once");
    let _ = remove_file("single_run/volume.fspf");
    info!("Updating policies...");
    create_command(prefix, force);
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
fn create_command(prefix : &String, force: bool) {
    // template for define OTP secret
//    let session_template = SESSION_TEMPLATE1;
//    let session_template2 = SESSION_TEMPLATE2;
//    let namespace_template = SESSION_TEMPLATE0;

    let (namespace_template, session_template, session_template2) = read_policies(&prefix);

    // create "volume"
    let _ = create_dir_all("single_run");
    let _ = create_dir_all("cosign_keys");

    let mut state : State = read_state("state.js"); // default: provide init state
    // retrieve MRENCLAVE from otp_image
    check_mrenclave(&mut state, "mrenclave", "otp_image", "otp_binary", force).expect("Failed to determine MRENCLAVE. Does image exist?"); // j, "mrenclave",
    state.namespace_hash = create_session(&state.namespace, &state.namespace_hash, &namespace_template, &state, force).expect("Creating namespace");

    let force = force || state.session_version != state.volume_version;  // check if we need to update the session?
    state.session_hash = create_session(&state.session, &state.session_hash, &session_template, &state, force).expect("Creating session");
    info!("Session hash = {}", state.session_hash);
    state.session_version = state.volume_version;

    let force = force || state.session_version2 != state.volume_version; // check if we need to update the session?
    state.session_hash2 = create_session(&state.session2, &state.session_hash2, &session_template2, &state, force).expect("Creating session2");
    info!("Session hash2 = {}", state.session_hash);
    state.session_version2 = state.volume_version;

    write_state(&state, "state.js");
}

fn add_authenticator(otp: Option<String>) {
    let state : State = read_state("state.js"); // default: provide init state
    let otp = get_otp(otp);

    info!("Got OTP {}", otp);
    let (code, stdout, stderr) = sh!(r#"docker run --rm -w "/root" -v "$PWD:/root" -e "SCONE_CAS_ADDR=scone-cas.cf" -e "SCONE_CONFIG_ID={}/otpqr@{}" otpqr:scone /bin/otpqr > qr.output"#, state.session2, otp);
    info!("Command: returns code: {}\n{}\n", code, stdout);
    if code != 0 {
        error!("ERROR: executing '/bin/otpqr.rs'. Code: {}\nError output:\n{}", code, stderr);
    } else {
        println!("Written QR code to file qrcode.svg.\n 1. Please 'open qrcode.svg' and scan qr code to initialize your authentication.\n 2. Remove qrcode.svg using: 'shred -n 3 -z -u qrcode.svg'\n")
    }
}


fn gen_keypair(otp: Option<String>) {
    let state : State = read_state("state.js"); // default: provide init state
    let otp = get_otp(otp);

    info!("Got OTP {}", otp);
    let (code, stdout, stderr) = sh!(r#"docker run --rm -w "/root" -v "$PWD:/root" -e "SCONE_CAS_ADDR=scone-cas.cf" -e "SCONE_CONFIG_ID={}/generate-key-pair@{}" cosign:scone  /go/bin/cosign > qr.output"#, state.session2, otp);
    info!("Command: returns code: {}\n{}\n", code, stdout);
    if code != 0 {
        error!("ERROR: executing 'cosign generate-key-pair'. Code: {}\nError output:\n{}", code, stderr);
    } else {
        println!("Generated key pair");
    }
}

fn get_otp(otp: Option<String>) -> String {
    if let Some(otp) = otp {
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
    }
}

fn sign_image(otp: Option<String>, image: String) {
    let state : State = read_state("state.js"); // default: provide init state

    let otp = get_otp(otp);
    let (code, stdout, stderr) = sh!(r#"docker run --rm -w "/root" -v /var/run/docker.sock:/var/run/docker.sock  -v "$HOME/.docker:/root/.docker" -v "$PWD/cosign_keys:/root/cosign_keys" -e "SCONE_CAS_ADDR=scone-cas.cf" -e "SCONE_CONFIG_ID={}/sign@{}" cosign:scone  /go/bin/cosign {} > qr.output"#, state.session2, otp, image);
    info!("Command: returns code: {}\n{}\n", code, stdout);
    if code != 0 {
        error!("ERROR: executing 'cosign generate-key-pair'. Code: {}\nError output:\n{}", code, stderr);
    } else {
        println!("Signed image");
    }
}

fn verify_image(otp: Option<String>, image: String) {
    let state : State = read_state("state.js"); // default: provide init state

    let otp = get_otp(otp);
    let (code, stdout, stderr) = sh!(r#"docker run --rm -w "/root" -v /var/run/docker.sock:/var/run/docker.sock  -v "$HOME/.docker:/root/.docker" -v "$PWD/cosign_keys:/root/cosign_keys" -e "SCONE_CAS_ADDR=scone-cas.cf" -e "SCONE_CONFIG_ID={}/verify@{}" cosign:scone  /go/bin/cosign {} > qr.output"#, state.session2, otp, image);
    info!("Command: returns code: {}\n{}\n", code, stdout);
    if code != 0 {
        error!("ERROR: executing 'cosign generate-key-pair'. Code: {}\nError output:\n{}", code, stderr);
    } else {
        println!("Verified Key");
    }
}
