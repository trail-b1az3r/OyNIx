# OyNIx website source

The website is maintained as separate source files by page. The build script combines each page with the shared shell and assets, then writes a static site for GitHub Pages.

## Page source layout

- `pages/index.html` — Overview page.
- `pages/downloads.html` — Stable, pre-release and legacy downloads page.
- `pages/docs/*.html` — One source file per documentation chapter. The header comment supplies its title, navigation label, order, icon and description.
- `../gallery/<extension-id>/` — Each extension has its own metadata, TypeScript and CSS. The build generates its gallery detail page and package download.
- `assets/site.css` — Shared layout, typography, responsive behavior and page components.
- `assets/site.js` — Shared progressive enhancements. Page-specific behavior is initialized only when that page's elements exist.
- `../scripts/build-site.py` — Static-site generator, shared page shell, route generation and extension gallery builder.

## Adding a documentation page

Create a new HTML file in `pages/docs/` and start it with metadata:

```html
<!--
title: New chapter
short: Chapter
order: 10
icon: book
lede: A short description shown below the page title.
description: Search and sharing description.
-->
<h2>First section</h2>
<p>Write the page content here.</p>
```

The generator reads the metadata, creates the documentation rail, extracts `h2` headings for the table of contents, and emits a separate route at `/docs/<filename>/`. Use `{{root}}` for links relative to the site root and `{{repo}}` for the GitHub repository URL.

## Building locally

From the repository root:

```sh
python3 scripts/build-site.py --out _site
```

The build uses Python's standard library. Open `_site/index.html` with a local static server to check routes and relative assets. GitHub Pages deployment is handled by `.github/workflows/pages.yml` when website source files change on `main`.

## Downloads behavior

The Downloads page obtains published release metadata from the public GitHub Releases API. Stable releases are rendered individually, pre-releases have a separate view, and versions v4.3 and older appear in the collapsed legacy archive. Asset buttons use each asset's `browser_download_url` directly; they must never link to a GitHub release-details page. The page should remain usable when JavaScript is unavailable by explaining that live release metadata requires the browser script.
