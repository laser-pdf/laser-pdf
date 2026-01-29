#!/usr/bin/env nu


def "main generate-images" [] {
    for file in (ls ./src/elements/snapshots/*.pdf) {
        print $"Converting: ($file | get name)"
        let stem = ($"./($file | get name)" | path parse | get stem)
        pdftoppm ($file | get name) $"./src/elements/snapshots/($stem)" -png
    }
}

def "main" [] {
    help main
}
