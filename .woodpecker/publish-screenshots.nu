#!/usr/bin/env nu
const SCREENSHOTS_DIR = path self ../screenshots
const COMMIT_MESSAGE = "docs: regenerate screenshots [skip ci]"

def push-url []: nothing -> string {
  let forge_host = $env.CI_FORGE_URL | url parse | get host
  $"https://oauth2:($env.CODEBERG_TOKEN)@($forge_host)/($env.CI_REPO).git"
}

def main [recordings: path]: nothing -> nothing {
  ^cp --no-preserve=mode ...(glob ($recordings | path join "*.gif")) $SCREENSHOTS_DIR
  if (^git status --porcelain -- $SCREENSHOTS_DIR | is-empty) {
    print "screenshots unchanged"
    return
  }
  ^git add -- $SCREENSHOTS_DIR
  ^git -c "user.name=lean-tui screenshots" -c user.email=ci@codeberg.org commit --message $COMMIT_MESSAGE
  let remote = push-url
  ^git fetch $remote main
  ^git rebase FETCH_HEAD
  ^git push $remote HEAD:refs/heads/main
}
