// Transforms the repo-root CHANGELOG.md (owned by the release pipeline) into a
// presentable docs page: human-readable dates, per-release GitHub links, stable
// anchors, and a "Latest" badge. Runs via the prestart/prebuild hooks; the
// generated docs/changelog.md is gitignored.
import { existsSync, readFileSync, writeFileSync } from 'node:fs'

const src = new URL('../../CHANGELOG.md', import.meta.url)
const dest = new URL('../docs/changelog.md', import.meta.url)
const zhHansDest = new URL('../i18n/zh-Hans/docusaurus-plugin-content-docs/current/changelog.md', import.meta.url)

const REPO_URL = 'https://github.com/AntigmaLabs/ante-preview'

const EN_MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
]

const formatDate = (iso, months) => {
  const [year, month, day] = iso.split('-').map(Number)
  return months ? `${months[month - 1]} ${day}, ${year}` : `${year}年${month}月${day}日`
}

const enFrontmatter = `---
slug: /changelog
sidebar_label: Changelog
description: Release notes for Ante
toc_max_heading_level: 2
---

`

const zhHansFrontmatter = `---
slug: /changelog
sidebar_label: 更新日志
description: Ante 发布说明
toc_max_heading_level: 2
---

`

const enIntro =
  `All notable changes to Ante, newest first. Every version is published as a ` +
  `[GitHub release](${REPO_URL}/releases), and you can install any of them with ` +
  '`ante update --version <V>`.'

const zhHansIntro =
  `Ante 的所有重要变更，按最新版本优先排列。每个版本都会发布为 ` +
  `[GitHub release](${REPO_URL}/releases)，也可以通过 ` +
  '`ante update --version <V>` 安装指定版本。'

const renderChangelog = ({ frontmatter, intro, title, months, latestLabel, releaseLabel }) => {
  const out = []
  let releases = 0

  for (const line of readFileSync(src, 'utf8').split('\n')) {
    // Release headers come from the release pipeline as "## vX.Y.Z - YYYY-MM-DD";
    // anything else passes through untouched.
    const release = line.match(/^## (v\S+) - (\d{4}-\d{2}-\d{2})\s*$/)
    if (!release) {
      out.push(line.trim() === '# Changelog' ? `# ${title}` : line)
      if (line.trim() === '# Changelog') out.push('', intro)
      continue
    }

    const [, version, date] = release
    releases += 1
    if (releases > 1) out.push('---', '')

    const anchor = version.toLowerCase().replace(/[^a-z0-9]+/g, '-')
    const meta = [
      releases === 1 ? `<span className="changelog-badge">${latestLabel}</span>` : '',
      `<time dateTime="${date}">${formatDate(date, months)}</time>`,
      `<a href="${REPO_URL}/releases/tag/${version}">${releaseLabel}</a>`,
    ].filter(Boolean).join('')

    out.push(`## ${version} {#${anchor}}`, '', `<p className="changelog-release-meta">${meta}</p>`)
  }

  return { content: frontmatter + out.join('\n'), releases }
}

const en = renderChangelog({
  frontmatter: enFrontmatter,
  intro: enIntro,
  title: 'Changelog',
  months: EN_MONTHS,
  latestLabel: 'Latest',
  releaseLabel: 'GitHub release',
})

writeFileSync(dest, en.content)
console.log(`synced CHANGELOG.md -> docs/changelog.md (${en.releases} releases)`)

if (existsSync(zhHansDest)) {
  const zhHans = renderChangelog({
    frontmatter: zhHansFrontmatter,
    intro: zhHansIntro,
    title: '更新日志',
    months: null,
    latestLabel: '最新',
    releaseLabel: 'GitHub 发布',
  })

  writeFileSync(zhHansDest, zhHans.content)
  console.log(`synced CHANGELOG.md -> i18n/zh-Hans/.../changelog.md (${zhHans.releases} releases)`)
}
