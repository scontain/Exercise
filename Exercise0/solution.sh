#!/usr/bin/env bash

# Exit on error and indicate which command caused this error exit
set -e 
trap 'last_command=$current_command; current_command=$BASH_COMMAND' DEBUG
trap 'echo "COMMAND \"${last_command}\" FAILED."' EXIT

# We want to add alias scone to file .bashrc in home directory

export BASHRC="$HOME/.bashrc"
export ALIAS="$HOME/.scone/alias"


function add_alias {
    echo "File $1 does not define alias scone. Adding now."
# append alias at the end of bashrc - assuming there is no early exit from bashrc (i.e., exits by executing last line of script)
    cat "$ALIAS" >> "$1"
}

# make sure this script is idempotent

mkdir -p "$HOME/.cas"
mkdir -p $HOME/.scone
touch "$HOME/.scone/state.env" || echo "Warning: cannot touch $HOME/.scone/state.env"
cat > "$ALIAS" <<EOF
# Created by 'SCONE add_alias' on $(date)
alias scone="docker run -it --rm \
    -v /var/run/docker.sock:/var/run/docker.sock \
    -v \"$HOME/.docker:/root/.docker\" \
    -v \"$HOME/.cas:/root/.cas\" \
    -v \"$HOME/.scone:/root/.scone\" \
    -v \"\$PWD:/root\" \
    -w /root \
    registry.scontain.com:5050/community/cli scone"
EOF

echo "Checking if file $BASHRC defines alias 'scone'."

# check if alias exists - add to bashrc if it does not

grep "alias scone" "$BASHRC" || add_alias "$BASHRC"

trap 'echo OK.' EXIT
