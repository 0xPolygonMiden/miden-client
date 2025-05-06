# Sets up Chrome sandbox as described in https://chromium.googlesource.com/chromium/src/+/main/docs/security/apparmor-userns-restrictions.md, since none is provided by default.
# See https://github.com/0xMiden/miden-client/issues/662 for reference.
CHROME_DEVEL_SANDBOX=/opt/google/chrome/chrome-sandbox
echo "CHROME_DEVEL_SANDBOX=$CHROME_DEVEL_SANDBOX" >> $GITHUB_ENV
sudo chown root:root $CHROME_DEVEL_SANDBOX
sudo chmod 4755 $CHROME_DEVEL_SANDBOX
