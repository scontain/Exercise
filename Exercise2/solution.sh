#!/usr/bin/env bash

# This script defines a new namespace
#  - ensures that only current user can create sessions in this namespace
#  - ensures that the name of the namespace is unique

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

#
# create session
#

function create_session {
    echo "  => namespace $NS does not seem to exist. Need to create."
    echo "- Creating session for namespace $NS"

    cat > session.yml <<EOF
name: \$NS
version: "0.2"
EOF


    export NAMESPACE_SESSION_HASH=`scone session create -e NS="$NS" session.yml || error "Error creating session $NS - maybe, session already exists?"` 
    echo "created session $NS with hash $NAMESPACE_SESSION_HASH"
}

#
# ensure that script is idempotent and 
# we do not forget the created namespace
#

echo "- Checking if we already created the name of the namespace"

unset NS
unset SESSION_HASH
touch state.env
source state.env

#
# check if NS is defined - if not, we create a new name for NS
#

if [[ "$NS" == "" ]] ; then 
    export NS="my_exercise-$USER-$RANDOM"
    echo "  => Has not existed. Creating name of a new namespace called \"$NS\""
else
    echo "  => The name of the namespace already exists - it is $NS"
fi

#
# check if namespace exists - if not, we create the namespace
#

echo "- Checking if namespace $NS already exists"

scone session read "$NS" > session_read.yml || create_session

echo "- Ensure that we store the up-to-date environment variables for the next execution"
echo "export NS=\"$NS\"" > state.env
echo "export NAMESPACE_SESSION_HASH=\"$NAMESPACE_SESSION_HASH\"" >> state.env

trap 'echo OK.' EXIT
