# Name and release route research

Historical decision research. The owner chose `sys-mon` and later narrowed
publication to a public source repository without a GitHub Release or binary
packages. The [specification](../wf-48ebf7-spec.md) and current Wayfinder M3
scope supersede the name and release-route recommendations below.

Researched 2026-10-05 for decision ticket `wf-48ebf7/t-61933c`. This is a recommendation, not an owner naming decision, trademark clearance, package implementation, or publication.

## Recommended name

**GRAUM**: **G**raphical **R**esource **A**ctivity and **U**sage **M**onitor. The words describe the app's graphical view of live system resource activity and use. Its lowercase program/repository spelling could be `graum`; the displayed name could be `GRAUM` with the expansion in the README and app metadata. These are proposed choices for the owner, not settled identifiers.

The [GitHub repository search for `graum` in repository names](https://github.com/search?q=graum+in%3Aname&type=repositories) returned 27 matching names when checked on 2026-10-05. Of the first 100 non-fork results examined through GitHub's repository search API, only [`bnwff/graum`](https://github.com/bnwff/graum) was an exact case-insensitive repository-name match, and its repository page says it is empty. This supports *reasonable software-name distinction* in the checked GitHub results; it does not establish exclusive use, future availability, or trademark rights. A separate web search for the exact expansion did not surface a product using it. Search indexing is incomplete and changes over time.

Rejected or weaker candidates, checked the same day:

| Candidate | Literal expansion | Direct collision evidence |
| --- | --- | --- |
| GURM | Graphical Usage and Resource Monitor | [`GURM`](https://github.com/capn-damo/GURM) is already the GIMP Unified Resource Manager, a Linux GUI plug-in. Reject. |
| GURD | Graphical Usage and Resource Dashboard | [`gurd`](https://github.com/Thepigcat76/gurd) is already a Linux C build tool; other exact-name projects also appeared in GitHub repository search. Reject. |
| GLURM | Graphical Linux Usage and Resource Monitor | [`glurm`](https://github.com/TolisSth/glurm) is already a Slurm web app manager; exact-name projects also appeared in GitHub repository search. Reject. |
| GRUMO | Graphical Resource Usage Monitor Overview | [`grumo`](https://github.com/papog/grumo) is already a graphing application. Reject, particularly because the purpose is close. |

**Open naming step:** Ask the owner whether to adopt GRAUM. Before creating public identifiers, repeat exact-name searches in GitHub, GNOME/Flathub listings and relevant package registries, and perform any legal trademark check the owner considers necessary. The crates.io lookup was unavailable during this research, so crate-name availability is unverified. A different repository owner can use the same basename on GitHub, but the selected package/application identifiers still need validation.

## Release packaging

**Recommended first route:** Publish source in the owner's personal *public* GitHub repository and attach versioned native Linux packages to GitHub Releases: a `.deb` for a specified Debian/Ubuntu baseline and an `.rpm` for a specified Fedora baseline, for each architecture actually built and tested. Provide source-build instructions for other GNOME/Linux distributions. This is a proposed release route; the exact distro versions and architectures depend on the selected GUI stack and CI coverage.

Why native first: Debian [defines `.deb` as its binary package format](https://www.debian.org/doc/debian-policy/ch-binary.html) and [records package dependencies](https://www.debian.org/doc/debian-policy/ch-relationships.html); RPM's [spec format](https://rpm.org/docs/4.20.x/manual/spec.html) supports package metadata and dependencies. Native installation runs against host interfaces under ordinary user permissions, which suits `/proc`, `/sys`, sensor and GPU discovery. It does not imply privileged operation; hardware readings that the user cannot access still need an unavailable state. Packages should install a desktop entry and icon according to the [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest-single/) and list runtime library requirements. GitHub [releases support downloadable binary assets and notes](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository).

**Flatpak is a later compatibility investigation, not the first release route.** Flatpak can distribute through [Flathub, a self-hosted repository, or a bundle](https://docs.flatpak.org/en/latest/publishing.html), but its [sandbox permissions reserve `/proc` and `/dev` from ordinary filesystem grants and expose only parts of `/sys` by default](https://docs.flatpak.org/en/latest/sandbox-permissions.html). A host-monitoring app may therefore see sandbox or partial host readings unless every required source and permission is demonstrated in a packaged build. Flathub also [requires minimal static permissions and source/offline builds](https://docs.flathub.org/docs/for-app-authors/requirements), with [submission review](https://docs.flathub.org/docs/for-app-authors/submission). This is a feasibility inference from the cited rules, not a claim that Flatpak can never work. Do not advertise Flatpak parity until host metric behavior is verified on GNOME Wayland and X11.

**AppImage is another later option.** It is a single-file distribution, but its [packaging guidance requires bundling non-baseline shared libraries correctly](https://docs.appimage.org/reference/best-practices.html); the [native-binary guide notes toolkit-specific deployment plugins](https://docs.appimage.org/packaging-guide/from-source/native-binaries.html). The selected Rust/GTK stack and actual target distros would determine whether this adds value after native packages.

## Public repository and MIT obligations

- Create the repository under the owner's personal account with **public** visibility. GitHub states that [public repositories are internet-accessible](https://docs.github.com/en/repositories/creating-and-managing-repositories/about-repositories); this ticket does not create one.
- Put the full MIT text in a root `LICENSE` file with the correct copyright holder and year. GitHub's [MIT license template/API](https://docs.github.com/en/rest/licenses/licenses) says the copyright and permission notice must accompany copies or substantial portions of the software. Record `MIT` in package metadata; review third-party dependencies and bundled assets separately for their own notices.
- Include a README with supported GNOME/Linux versions and architectures, install and uninstall steps per package, source-build requirements, metric availability and permissions, and the limits of hardware verification. The final supported matrix must come from tests, not this research.
- Use a version tag and matching GitHub Release, release notes, asset filenames that identify distro/architecture, and SHA-256 checksums. GitHub's [release asset API](https://docs.github.com/en/rest/releases/assets) exposes uploaded assets and SHA-256 digests; publishing them plainly in release notes makes verification practical.

## Unresolved questions for the owner and implementation session

1. Does the owner approve **GRAUM** as the product name? This report recommends it but does not decide it.
2. Which Debian/Ubuntu and Fedora versions, plus CPU architectures, can CI and hands-on QA actually cover? Set the supported package matrix from that evidence.
3. After native release, is Flatpak distribution worth investigating with a real host-metric permission test? Do not assume the sandbox can satisfy the full telemetry contract.

Hardware not present locally and an actual weeks-long soak are not release gates under the confirmed destination. Release notes should identify unverified vendor/device paths and the bounded reliability evidence collected instead.
