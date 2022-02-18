#!/usr/bin/env bash
#
# This script checks the hash value of a policy by
# comparing the hash value stored by the 

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
# check if we have the correct hash value 
#

echo "- Retrieving the name of the namespace from excercise 2"

unset NS
unset SESSION_HASH
source ../Exercise2/state.env

if [[ "$NS" == "" ]] ; then 
    error "No namespace defined"
fi

#
# compute the hash value of the session
#

echo "- Retrieving session of namespace $NS"

scone session read "$NS" > session_read.yml || error "Namespace $NS does not exist - or we do not have access. Try to execute exercise 2 again."

export COMPUTED_NAMESPACE_HASH=$(scone session verify session_read.yml | tr -d '[:space:]') || error "Error when retrieving the hash of the session $NS."

#
# check if we have the correct hash value 
#

if [[ "$NAMESPACE_SESSION_HASH" == "$COMPUTED_NAMESPACE_HASH" ]] ; then
    echo "Namespace hash is as expected: $COMPUTED_NAMESPACE_HASH"
else
    echo "Namespace hash has changed: was \"$NAMESPACE_SESSION_HASH\" and it is now \"$COMPUTED_NAMESPACE_HASH\""
fi

cat > state.env <<EOF
export NS="${NS}"
export NAMESPACE_SESSION_HASH="${COMPUTED_NAMESPACE_HASH}"
EOF

#
# try to retrieve the session with the hash
#

# extract cas cert
scone cas show-certificate > cas_cert.crt

# extract client certificate
printf "\n$(cat $HOME/.cas/config.json | jq .identity | tr -d '\"')" > client.pem 

# get the session specifying session name and session hash

curl --cacert  cas_cert.crt  --connect-to cas:8081:scone-cas.cf:8081 --cert client.pem  https://cas:8081/v1/sessions/${NS}?hash=${COMPUTED_NAMESPACE_HASH} | jq -r .session > read_session.txt

echo "Retrieved policy $NS using its name and hash: $COMPUTED_NAMESPACE_HASH"

cat read_session.txt

trap 'echo OK.' EXIT
