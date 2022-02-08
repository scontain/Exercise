#! /usr/bin/env bash
#
# Attest SCONE CAS
#

# Exit on error and indicate which command caused this error exit
set -e
trap 'second_last=$last_command; last_command=$current_command; current_command=$BASH_COMMAND' DEBUG
trap 'echo "COMMAND \"${last_command}\" (or \"${second_last}\") FAILED."' EXIT


#
# error exit: print error and exit
#

function error {
    printf "FATAL ERROR: $1.\nCAUSED BY:"
    exit 1
}

#
# define "scone" alias .. needs to be defined before any functions that might use this alias
#

shopt -s expand_aliases
export ALIAS="$HOME/.scone/alias"
source "$ALIAS"
type -a scone || error "alias 'scone' undefined. Please add this to your .bashrc first."

# Let us attest CAS
#
# Production CAS is signed and we can attest using MrSigner but the public debug CAS is not signed. 
# We need to determine MRENCLAVE of CAS

CAS_MRENCLAVE=$(curl https://sconedocs.github.io/public-CAS/ | grep "scone cas attest" | tail -1 | awk '{ print $9 }')
echo "Extracted MRENCLAVE of CAS from website 'sconedocs.github.io': $CAS_MRENCLAVE"
scone cas attest -G  --mrenclave $CAS_MRENCLAVE  --only_for_testing-ignore-signer --only_for_testing-debug scone-cas.cf || error "Attestation of CAS failed!"

# Let us extract the cas certificate

scone cas show-certificate > cas_cert.crt

# Retrieving public value from $CAS_ADDR

export session_name="example_session"
export value_name="example_value"

curl --cacert  cas_cert.crt  --connect-to cas:8081:$CAS_ADDR:8081 https://cas:8081/v1/values/session=${session_name},secret=${value_name} > values.json

export VALUE=$( cat values.json | jq .value | tr -d '"')

#
# check if we have retrieved the correct public value 
#

scone cas show-certificate > cas_cert.crt


if [[ "$VALUE" == "42" ]] ; then
    echo "Value is as expected: $VALUE"
else
    echo "Value is not as expected: expected \"42\" and got \"$VALUE\""
fi

trap 'echo OK.' EXIT
