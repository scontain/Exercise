# OTP Policy Generation

In this assignment, we look into how to use OTPs to authorize users that want to run
certain commands. To clean up the code, we moved the boilerplate code into a Rust crate
`scone_cli` - which is a wrapper around the SCONE CLI commands.

Since it is difficult to write clean and robust code with `bash`, we move to a different 
scripting language: `rust-script`. It has the features of scripting languages - one can
try to understand and modify the code. Since this is basically Rust code, it is very 
simple to actually use this later as normal rust code.

## Prerequisite 

### ``rust-script``
In case you already have Rust installed, installing ``rust-script`` is a one liner:

```bash
cargo install rust-script
```

For more details, please have a look at <https://crates.io/crates/rust-script>.

### ``docker``

We assume that you have ``docker`` installed on your development machine.

### ``scone``

Actually, you do not need to install anything regarding ``scone``, you do not even need
an SGX-enabled CPU. However, you need to have access to some container images. These
will become part of the ``scone`` community edition. Just sign up for a free account (see <https://sconedocs.github.io/registry/).

## Executing this program 

Just run in this directory, 

```bash
 ./otp_policy.rs --help
 ```

to get some overview of the different commands.  You  

