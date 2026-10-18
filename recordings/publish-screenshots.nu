#!/usr/bin/env nu
const COMMIT_MESSAGE = "docs: regenerate screenshots [skip ci]"

def screenshots-dir []: nothing -> path {
  ^git rev-parse --show-toplevel | str trim | path join screenshots
}

def push-url []: nothing -> string {
  let forge_host = $env.CI_FORGE_URL | url parse | get host
  $"https://oauth2:($env.CODEBERG_TOKEN)@($forge_host)/($env.CI_REPO).git"
}

def main [recordings: path]: nothing -> nothing {
  let screenshots = screenshots-dir
  ^cp --no-preserve=mode ...(glob ($recordings | path join "*.gif")) $screenshots
  if (^git status --porcelain -- $screenshots | is-empty) {
    print "screenshots unchanged"
    return
  }
  ^git add -- $screenshots
  ^git -c "user.name=lean-tui screenshots" -c user.email=ci@codeberg.org commit --message $COMMIT_MESSAGE
  let remote = push-url
  ^git fetch $remote main
  ^git rebase FETCH_HEAD
  ^git push $remote HEAD:refs/heads/main
}
