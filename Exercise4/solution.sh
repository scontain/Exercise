#!/usr/bin/env bash
#
# This script read the namespace
#  - and updates the namespace to a new version (0.3 instea)
#

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
# Retrieve a value from a given file ($1) and Key ($2)
#

yaml() {
    python3 -c "import yaml;print(yaml.safe_load(open('$1'))$2)"
}


#
# create updated namespace session
#

function create_session {
    echo "- Updating namespace $NS"
    echo "- Creating session for updated namespace $NS"

    cat > session.yml <<EOF
name: \$NS
version: "0.3"
predecessor: \$PREDECESSOR

access_policy:
  read:
    - CREATOR
  update:
    - CREATOR
  create_sessions:
    - CREATOR

EOF

    scone session check -e NS="$NS" -e PREDECESSOR="$COMPUTED_NAMESPACE_HASH" session.yml || error "Error checking session $NS?!!"

    export NAMESPACE_SESSION_HASH=$(scone session create -e NS="$NS" -e PREDECESSOR="$COMPUTED_NAMESPACE_HASH" session.yml || error "Error creating session $NS - maybe, session already exists?")

    echo "created session $NS with hash $NAMESPACE_SESSION_HASH"
}

#
# check if we have the correct hash value 
#

echo "- Retrieving the name of the namespace from excercise 3"

unset NS
unset SESSION_HASH
source ../Exercise3/state.env

if [[ "$NS" == "" ]] ; then 
    error "No namespace defined"
fi

#
# compute the hash value of the session
#

echo "- Retrieving session of namespace $NS"

scone session read "$NS" > session_read.yml || error "Namespace $NS does not exist - or we do not have access."

export COMPUTED_NAMESPACE_HASH=$(scone session verify session_read.yml) || error "Error when retrieving the hash of the session $NS."

#
# check if we have the correct hash value 
#

if [[ "$NAMESPACE_SESSION_HASH" == "$COMPUTED_NAMESPACE_HASH" ]] ; then
    echo "Namespace hash is as expected: $COMPUTED_NAMESPACE_HASH"
else
    echo "Namespace hash has changed: was \"$NAMESPACE_SESSION_HASH\" and it is now \"$COMPUTED_NAMESPACE_HASH\""
fi

VERSION=$(yaml session_read.yml "['version']")
if [[ "$VERSION" != "0.3" ]] ; then
    echo "Namespace version is: $VERSION .. upgrading to 0.3"
    create_session
else
    echo "Namespace has already been upgraded to version 0.3"
fi



cat > state.env <<EOF
export NS="${NS}"
export NAMESPACE_SESSION_HASH="${COMPUTED_NAMESPACE_HASH}"
EOF


# extract cas cert
scone cas show-certificate > cas_cert.crt

# extract client certificate
printf "\n$(cat $HOME/.cas/config.json | jq .identity | tr -d '\"')" > client.pem 

# get the session specifying session name and session hash

curl --cacert  cas_cert.crt  --connect-to cas:8081:scone-cas.cf:8081 --cert client.pem  https://cas:8081/v1/sessions/${NS} 2> /dev/null | jq -r .session > read_session.txt
cat read_session.txt

# retrieve all sessions that preceeded this session

PRED=$(yaml read_session.txt "['predecessor']")
while [[ "$PRED" != "None" ]] ; do
    echo "PREDECESSOR is ${PRED}"
    curl --cacert  cas_cert.crt  --connect-to cas:8081:scone-cas.cf:8081 --cert client.pem  https://cas:8081/v1/sessions/${NS}?hash=${PRED} 2> /dev/null | jq -r .session > read_session.txt
    PRED=$(yaml read_session.txt "['predecessor']")
done
echo "No more PREDECESSORs"

trap 'echo OK.' EXIT
