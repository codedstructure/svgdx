#!/bin/sh
#
# Downloads vendored CodeMirror 5 files.
#
# See https://codemirror.net/5/LICENSE for the (MIT) license under which
# CodeMirror is used in this project.

set -eu

OUT_DIR="cm5"
URL_BASE="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.18"

FILE_LIST="\
    codemirror.min.js \
    codemirror.min.css \
    mode/xml/xml.min.js \
    addon/display/autorefresh.min.js \
    addon/fold/foldgutter.js \
    addon/fold/foldcode.js \
    addon/fold/foldgutter.min.css \
    addon/fold/xml-fold.min.js \
"

for FILE in $FILE_LIST ; do
    FULL_URL="${URL_BASE}/${FILE}"
    FOLDER="$(dirname $FILE)"
    mkdir -p "${OUT_DIR}/${FOLDER}"
    echo "Downloading $FILE..."
    curl -s -o "${OUT_DIR}/${FILE}" "$FULL_URL"
done

