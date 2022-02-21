#!/bin/bash

echo '
# Screencast for OTP assignments
# let us change directory to the reference solution
cd Section_OTP/otp_policies

# let us look at the directory first
ls -l

# Ok, we have a the rust-script and a README
# Let us ensure that rust-script is installed.
cargo install rust-script

# Next, lets see what commands otp-policies.rs implements:

./otp_policy.rs help

# Let us get more info about command create

./otp_policy.rs create --help

# Let us execute a create

./otp_policy.rs create

# Ok, no error messages. Let us see what changed.

ls -l

# We see there is a new directory single_run and a state.js file

ls single_run

# Ok, single_run is empty. Let us look at the state:

cat state.js

# Ok there are all information in the clear text. We should protect this
# even when running on a trusted computer!
# We do this in a later assignment

# Let us look at the namespace session first

scone session read $(cat state.js | jq -r .namespace)

# This looks fine, lets look at the session next:

scone session read $(cat state.js | jq -r .session)

# Ok - there is no use of OTPs in this session.

# Let us look at the help for generating the QR first:

./otp_policy.rs gen-qr-code  --help

# Next, we generate the QR code

./otp_policy.rs gen-qr-code

# You can look at the output as follows

open qrcode.svg

# Let us get rid of the qrcode.svg file:

shred -n 3 -z -u qrcode.svg

# Let us try again to generate the QR code file

./otp_policy.rs gen-qr-code

# We can generate a new QR code by providing a valid OTP

# Let us generate an OTP ... using a simple script and a secret from
# the unencrypted state.js

./print_otp.rs $(cat state.js | jq -r .secret)

# We can now try to do this using the OTP from

./otp_policy.rs add-authenticator --ootp $(./print_otp.rs $(cat state.js | jq -r .secret))

# Let us assume that we lost our authenticator, let us roll forward

cp state.js state.old
./otp_policy.rs roll-forward --force

# Let us look what changed:

diff state.js state.old

# We see that the session hashes have changed and that the OTP secret was changed

# Next, we generate the new QR code

./otp_policy.rs gen-qr-code

# Ok that worked - if we run again, this should fail

./otp_policy.rs gen-qr-code

# and indeed, it failed.

# Ok this fails as expected!
# Let us look at the files in single_run:

ls -l single_run/

# Let us removed file "once" and reexecute:

rm -f single_run/once
./otp_policy.rs gen-qr-code

# This fails since "once" is missing.
# Let us also remove file "volume.fspf" that contains the protection data!

rm -f single_run/once single_run/volume.fspf
./otp_policy.rs gen-qr-code

# Ok this fails since file system protection file was removed

# Let us try a rollback attack. To do so, we reinitialize this:

./otp_policy.rs roll-forward --force

ls -l single_run

# Ok, there is no metadata file yet in "single_run"
# We added a command that creates a file in the directory

./otp_policy.rs test-qr-code
ls -l single_run

# Ok we have a test and the metadata file
# lets copy this

cp -r single_run/ rollback

# let us generate a QR code

./otp_policy.rs gen-qr-code

# ok that worked. It generated the files:

ls -l single_run

# let us remove the whole directory

rm -rf single_run/

# Let us do a rollback

mv rollback single_run

# Now we are at state before we generated the QR code
# Let us try again to generate the QR code

./otp_policy.rs gen-qr-code


# OK, SCONE aborts with "the supplied tag is not fresh - suspected replay attack"
# Which is exactly what happened! Well done SCONE!

exit
' > screencast.txt
rm -rf otp_policies/state.js otp_policies/state.old otp_policies/single_run otp_policies/qr.output otp_policies/qrcode.svg otp_policies/screencast_wrong.txt otp_policies/test.svg
cd ..
cat Section_OTP/screencast.txt | Section_OTP/stream.sh |  asciinema rec -t "Section OTP Assignments" -i 1 -y
