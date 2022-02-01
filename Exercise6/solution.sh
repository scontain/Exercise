#!/usr/bin/env bash
# This script creates a session for the SCONE CLI


#
# Exit on error and indicate which command caused this error exit
#

set -e -x
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

BASHRC="$HOME/.bashrc"
source "$BASHRC"
shopt -s expand_aliases
type -a scone || error "alias 'scone' undefined. Please add this to your .bashrc first."


# load state from last exercise

function load_predecessor_state {
    unset NS
    unset SESSION_HASH
    source ../Exercise3/state.env
}

# load state of this exercise

function load_state {
    touch state.env
    source state.env
    if [[ "$CLI_SESSION" == "" ]] ; then
        export CLI_SESSION="$NS/cli"
    fi
}

# store state of this code

function store_state {
    echo "export NS=\"$NS\"" > state.env
    echo "export NAMESPACE_SESSION_HASH=\"$NAMESPACE_SESSION_HASH\"" >> state.env
    echo "export CLI_SESSION=\"$CLI_SESSION\"" >> state.env
    echo "export CLI_SESSION_HASH=\"$COMPUTED_SESSION_HASH\"" >> state.env
}


# create a session

function create_session {
    SESSION="$1"
    SESSION_FILE="$2"
    >&2 echo "- Creating session $SESSION from session file $SESSION_FILE"

    scone session create -e NS="$NS" session.yml || error "Error creating session $SESSION"
}


# check session hash and create if needed

function check_session {
    SESSION="$1"
    SESSION_FILE="$2"
    SESSION_HASH="$3"

    >&2 echo "- Retrieving session $SESSION"

    type -a scone

    scone session read "$SESSION" > session_read.yml || SESSION_HASH=$(create_session "$SESSION" "$SESSION_FILE")

    export COMPUTED_SESSION_HASH=$(scone session verify session_read.yml) || error "Error when retrieving the hash of the session $NS."

    if [[ "$SESSION_HASH" == "$COMPUTED_SESSION_HASH" ]] ; then
        >&2 echo "Session hash is as expected: $SESSION_HASH"
    else
        >&2 echo "Session hash has changed: expected $SESSION_HASH and it is now $COMPUTED_SESSION_HASH"
    fi

}


## create session if it does not exist

function create_session_file {
    FILE="$1"
    cat > "$FILE" <<EOF
name: $NS/sconecli
version: "0.3"

access_policy:
  read:
    - CREATOR
  update:
    - CREATOR

services:
   # read session - 
   - name: read
     command:  scone session read @1
     attestation:
        - mrenclave:
          - $MRENCLAVE
     image_name: scone_cli

#
# for production use: remove debug mode and hyperthreading!
#
security:
  attestation:
    tolerate: [debug-mode, hyperthreading]


images:
   - name: scone_cli
     injection_files:
      - path: /root/.cas/config.json
        content: |
EOF
    cp .cas/config.json config.json
    sed -i -e 's/^/          /' config.json
    cat config.json >>session.yml
}

# execute the steps

load_predecessor_state
load_state

export MRENCLAVE="0fd62302e237bb4b4ccf8c7f312ec31ef9a5e79e477c2bceda9e801dd4b154b3"
SESSION_FILE="session.yml"
create_session_file "$SESSION_FILE"
scone session check "$SESSION_FILE"
check_session  "$CLI_SESSION" "$SESSION_FILE" "$CLI_SESSION_HASH"

trap 'echo OK.' EXIT
