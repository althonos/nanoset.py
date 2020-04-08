#!/bin/sh

. $(dirname $0)/functions.sh

# --- Update GitHub release notes --------------------------------------------

export GEM_PATH="$(ruby -r rubygems -e 'puts Gem.user_dir')"
export PATH="${GEM_PATH}/bin:$PATH"

log Installing chandler gem
gem install --user-install chandler

log Updating GitHub release notes
chandler push --github="$TRAVIS_REPO_SLUG" --changelog="CHANGELOG.md"

# --- Deploy to PyPI ---------------------------------------------------------

if [ "$TRAVIS_OS_NAME" = "osx" ]; then
  log Activating pyenv
  eval "$(pyenv init -)"
  pyenv shell $(pyenv versions --bare)
fi

log Deploying to PyPI
python3 -m twine upload --skip-existing dist/*.whl dist/*.tar.gz

# --- Deploy to crates.io ----------------------------------------------------

if [ "$TRAVIS_PYTHON_VERSION" = "3.7" ]; then
  log Deploying to \`crates.io\`
  cargo publish --token $CRATES_IO_TOKEN
fi
