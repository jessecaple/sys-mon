# Resource monitor name shortlist

Researched 2026-10-05 for decision ticket `wf-48ebf7/t-6eece2`. The owner rejected GRAUM because "activity AND usage is strange" and asked for more names. This report recommends choices; it does not name the product, clear trademarks, register package identifiers, or change the [separate release-packaging report](name-and-release-route.md).

## Best choices

1. **GLORUM**: **G**raphical **L**inux **O**verview of **R**esource **U**sage **M**etrics. Spoken "glor-um." Each word has a job: the interface is graphical, Linux is the platform, one screen is the overview, and resource-usage metrics are what it shows. [GitHub's current repository-name search](https://api.github.com/search/repositories?q=glorum+in%3Aname+fork%3Afalse) returned zero matches on 2026-10-05. [Debian's package-name search](https://packages.debian.org/search?keywords=glorum&searchon=names&suite=all&section=all) and [Flathub's app search](https://flathub.org/apps/search?q=glorum) returned no results. `cargo search glorum --limit 3` also returned no entries. **Recommended first.** Its six letters take a moment to learn, but the expansion is literal and natural.
2. **VOLOM**: **V**isual **O**verview of **L**inux **O**perational **M**etrics. Spoken "voh-lum." It is compact and broadly covers CPU, GPU, memory, storage, network and temperatures. [GitHub's search](https://api.github.com/search/repositories?q=volom+in%3Aname+fork%3Afalse) found one exact repository, [`volom/volom`](https://github.com/volom/volom), whose page says it is empty, not a monitor. [Debian](https://packages.debian.org/search?keywords=volom&searchon=names&suite=all&section=all) and [Flathub](https://flathub.org/apps/search?q=volom) returned no name results; `cargo search volom --limit 3` returned no entries. **Recommended second, with a resemblance caveat:** [`vloom`](https://github.com/victory-sokolov/vloom) is an existing Rust screen-recording CLI and [crates.io package](https://crates.io/crates/vloom). VOLOM and VLOOM are different strings and products but could be misread.

The absence of a result in these services is limited evidence about their current indexes, not proof that a name is exclusive or legally available. Repeat searches before publication, and have the owner choose the final name.

## Other natural expansions checked

| Candidate | Expansion | Current direct collision evidence | Assessment |
| --- | --- | --- | --- |
| **GLORAM** | **G**raphical **L**inux **O**verview of **R**esource **A**vailability **M**etrics | [GitHub repository-name search](https://api.github.com/search/repositories?q=gloram+in%3Aname+fork%3Afalse) returned no exact match among eight substring hits. | Distinct, but "availability" puts less emphasis on live utilization; backup only. |
| **GLOR** | **G**raphical **L**inux **O**verview of **R**esources | [`lpil/glor`](https://github.com/lpil/glor) is an existing browser-audio app; [`ETomberg391/Glor`](https://github.com/ETomberg391/Glor) is an Android app. | Clear expansion, already used by software. |
| **GOLT** | **G**raphical **O**verview of **L**inux **T**elemetry | [`chowder/golt`](https://github.com/chowder/golt) is a Linux game launcher; [`iryndin/golt`](https://github.com/iryndin/golt) is a load-test tool. | Clear expansion, too many exact software uses. |
| **GRUM** | **G**raphical **R**esource **U**sage **M**onitor | [`nyx0/Grum`](https://github.com/nyx0/Grum) is a spam bot; [`harryjo97/GruM`](https://github.com/harryjo97/GruM) names a graph-generation model. | Direct four-word expansion, exact software-name collisions. |
| **VLOOM** | **V**isual **L**inux **O**verview of **O**perational **M**etrics | [`victory-sokolov/vloom`](https://github.com/victory-sokolov/vloom) is a Rust video-recording CLI; [`msuvakovic/VLoom`](https://github.com/msuvakovic/VLoom) is a game engine. | Natural expansion, exact software collisions. |
| **VORAL** | **V**isual **O**verview of **R**esources **A**cross **L**inux | [Voral's own site](https://voral.ai/) presents an active medical-practice software platform under this name. | Natural expansion, active product collision. |
| **GOLOM** | **G**raphical **O**verview of **L**inux **O**perational **M**etrics | [Golom's code and public audit references](https://github.com/code-423n4/2022-07-golom) identify an existing NFT marketplace; [GitHub search](https://api.github.com/search/repositories?q=golom+in%3Aname+fork%3Afalse) also found exact project names. | Natural expansion, software-name collision. |

The expansions omit linking words such as "of" from the initials, as ordinary acronyms often do. Unlike GRAUM, none joins *activity* and *usage* as parallel concepts. "Resource usage metrics" names the measured values; "operational metrics" is a broader category.

## Search limits and next step

GitHub results above were checked through the live repository search API with `in:name fork:false`; exact matches were compared case-insensitively. The first 100 results were examined for each query, so high-volume searches such as GLOR, GOLT, VLOOM and GOLOM may have further exact matches. Debian's package-name and Flathub's app searches were checked for the two recommendations. `cargo search` uses the crates.io index, but a crate can appear after this check or under a related name. General web search did not surface a monitor called GLORUM or VOLOM; search engines are incomplete. This was a software-name screen, not a trademark search.

**Owner choice needed:** choose GLORUM or VOLOM, or request another naming direction. The main session should ask the owner directly and record that decision in the appropriate Wayfinder decision ticket before implementation assigns app IDs, package names or repository names.
