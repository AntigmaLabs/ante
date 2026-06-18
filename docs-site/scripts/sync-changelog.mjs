// Transforms the repo-root CHANGELOG.md (owned by the release pipeline) into a
// presentable docs page: human-readable dates, per-release GitHub links, stable
// anchors, and a "Latest" badge. Runs via the prestart/prebuild hooks; the
// generated docs/changelog.md is gitignored.
import { readFileSync, writeFileSync } from 'node:fs'

const src = new URL('../../CHANGELOG.md', import.meta.url)
const dest = new URL('../docs/changelog.md', import.meta.url)

const REPO_URL = 'https://github.com/AntigmaLabs/ante-preview'

const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
]

const formatDate = (iso) => {
  const [year, month, day] = iso.split('-').map(Number)
  return `${MONTHS[month - 1]} ${day}, ${year}`
}

const frontmatter = `---
slug: /changelog
sidebar_label: Changelog
description: Release notes for Ante
toc_max_heading_level: 2
---

`

const intro =
  `All notable changes to Ante, newest first. Every version is published as a ` +
  `[GitHub release](${REPO_URL}/releases), and you can install any of them with ` +
  '`ante update --version <V>`.'

const out = []
let releases = 0

for (const line of readFileSync(src, 'utf8').split('\n')) {
  // Release headers come from the release pipeline as "## vX.Y.Z - YYYY-MM-DD";
  // anything else passes through untouched.
  const release = line.match(/^## (v\S+) - (\d{4}-\d{2}-\d{2})\s*$/)
  if (!release) {
    out.push(line)
    if (line.trim() === '# Changelog') out.push('', intro)
    continue
  }

  const [, version, date] = release
  releases += 1
  if (releases > 1) out.push('---', '')

  const anchor = version.toLowerCase().replace(/[^a-z0-9]+/g, '-')
  const meta = [
    releases === 1 ? '<span className="changelog-badge">Latest</span>' : '',
    `<time dateTime="${date}">${formatDate(date)}</time>`,
    `<a href="${REPO_URL}/releases/tag/${version}">GitHub release</a>`,
  ].filter(Boolean).join('')

  out.push(`## ${version} {#${anchor}}`, '', `<p className="changelog-release-meta">${meta}</p>`)
}

writeFileSync(dest, frontmatter + out.join('\n'))
console.log(`synced CHANGELOG.md -> docs/changelog.md (${releases} releases)`)
