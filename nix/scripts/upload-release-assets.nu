#!/usr/bin/env nu

def main [assets_dir: path]: nothing -> nothing {
  cd $assets_dir
  ^sha256sum ...(glob *.tar.gz | path basename) | save --force sha256sum.txt
  ^tea login add --name codeberg --url $env.CI_FORGE_URL --token $env.CODEBERG_TOKEN
  ^tea releases assets create --login codeberg --repo $env.CI_REPO $env.CI_COMMIT_TAG ...(glob * | path basename)
}
