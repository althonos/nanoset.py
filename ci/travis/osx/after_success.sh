#!/bin/sh -e

. $(dirname $(dirname $0))/functions.sh

# --- Wheels -----------------------------------------------------------------

if [ ! -z "$TRAVIS_TAG" ]; then
  log Building wheel
  $PYTHON setup.py sdist bdist_wheel
fi
