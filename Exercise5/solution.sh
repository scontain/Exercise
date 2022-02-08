#!/usr/bin/env bash

# This script defines a new namespace
#  - we create a session and update the session to upload a new 
#  - export a public value
#  - read the public value via a curl

#
# Exit on error and indicate which command caused this error exit
#

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

unset SESSION_HASH

#
# create session
#

function create_session {
    echo "  => session $SESSION does not seem to exist. Need to create."
    echo "- Creating session $SESSION"

    cat > session.yml <<EOF
name: $SESSION
version: "0.3"

secrets:
  - name: public_value
    kind: ascii
    value: "42"
    export_public: true
  - name: private_value
    kind: ascii
    value: "007"
EOF
    scone session check session.yml || error "Session seems to be incorrect"
    export SESSION_HASH=`scone session create session.yml || error "Error creating session $SESSION - maybe, session already exists?"` 
    echo "created session $SESSION with hash $SESSION_HASH"
}

#
# ensure that script is idempotent and 
# we do not forget the created namespace
#

echo "- Checking if we already created the name of the namespace"

source ../Exercise4/state.env


#
# retrieve the public value
#

#
# attesting cas
#
# Typically, already done in a previous exercise and hence, we skip it here.
#

function attest_cas {
    export CAS_ADDR=scone-cas.cf
    export CAS_MRENCLAVE=$(curl https://sconedocs.github.io/public-CAS/ | grep "scone cas attest" | tail -1 | awk '{ print $9 }')

    echo "$CAS_ADDR has MRENCLAVE=$CAS_MRENCLAVE"

    echo "Attesting $CAS_ADDR"

    scone cas attest -G   --mrenclave $CAS_MRENCLAVE  --only_for_testing-ignore-signer --only_for_testing-debug $CAS_ADDR || error "Error attesting $CAS_ADDR"
    scone cas show-certificate > cas_cert.crt
}

function get_mTLS_certs {
    # extract cas cert
    scone cas show-certificate > cas_cert.crt

    # extract client certificate
    printf "\n$(cat $HOME/.cas/config.json | jq .identity | tr -d '\"')" > client.pem 
}

#
# check if the session exists - if not, we create the session
#

export SESSION="$NS/private_public_values"
echo "- Checking if session $SESSION already exists"
if scone session read "$SESSION" > session_read.yml ; then
    export SESSION_HASH=$(scone session verify session_read.yml)
else
    create_session
fi

get_mTLS_certs

echo "Retrieving public value from cas"

export value_name="public_value"

curl --cacert  cas_cert.crt  --connect-to cas:8081:scone-cas.cf:8081 https://cas:8081/v1/values/session=${SESSION},secret=${value_name}  2> /dev/null > values.json || error "Failed to retrieve public value"
export VALUE=$( cat values.json | jq .value | tr -d '"')

#
# check if we have retrieved the correct public value 
#

if [[ "$VALUE" == "42" ]] ; then
    echo "Value is as expected: $VALUE"
else
    echo "Value is not as expected: expected \"42\" and got \"$VALUE\""
fi


echo "Retrieving private value from cas"

export value_name="private_value"

export BR=$(curl -I --cacert  cas_cert.crt  --connect-to cas:8081:scone-cas.cf:8081 https://cas:8081/v1/values/session=${SESSION},secret=${value_name}  2> /dev/null | grep "405" | wc -l | xargs)
if [[ "$BR" != "1" ]] ; then
    error "Expected response \"405\" retrieving private secret. Got \"$BR\""
else
    echo "Got error 405 - as expected."
fi

echo "Let's try to read with client cert - should result in the same error code since we do not have access to the values!"

export BR=$(curl -I --cacert  cas_cert.crt  --cert client.pem  --connect-to cas:8081:scone-cas.cf:8081 https://cas:8081/v1/values/session=${SESSION},secret=${value_name}  2> /dev/null | grep "405" | wc -l | xargs)
if [[ "$BR" != "1" ]] ; then
    error "Expected response \"405\" retrieving private secret. Got \"$BR\""
else
    echo "Got error 405 - as expected."
fi

cat > state.env <<EOF
export NS="${NS}"
export SESSION="${SESSION}"
export SESSION_HASH="${SESSION_HASH}"
EOF

trap 'echo OK.' EXIT
