# coding: utf-8

import configparser
import os
import textwrap
import toml
import packaging.version
import semantic_version

PROJDIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Extract project metadata
manifest = toml.load(os.path.join(PROJDIR, "Cargo.toml"))
setupcfg = configparser.ConfigParser()
setupcfg.read(os.path.join(PROJDIR, "setup.cfg"))

py_version = setupcfg.get("metadata", "version")
rs_version = manifest["package"]["version"]

# Get different versions
if os.getenv("TRAVIS") == "true":
    ci_version = os.getenv("TRAVIS_TAG")
elif os.getenv("APPVEYOR") == "True":
    ci_version = os.getenv("APPVEYOR_REPO_TAG_NAME")
else:
    raise RuntimeError("could not detect CI environment")
if ci_version is None:
    raise RuntimeError("could not find release tag")

# Check all versions are the same (duh)
if not py_version == rs_version == ci_version.lstrip('v'):
    raise RuntimeError(textwrap.dedent(
            """versions differ:
            - setup.cfg:  {}
            - Cargo.toml: {}
            - git tag:    {}
            """.format(py_version, rs_version, ci_version)
        )
    )

# Check all versions are valid for the respective distribution sites
if not isinstance(packaging.version.parse(py_version), packaging.version.Version):
    raise RuntimeError("not a PyPI version: {}".format(py_version))
if not semantic_version.validate(rs_version):
    raise RuntimeError("not a semver: {}".format(rs_version))
