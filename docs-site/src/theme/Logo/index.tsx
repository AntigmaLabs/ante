import React from 'react'
import Link from '@docusaurus/Link'
import useBaseUrl from '@docusaurus/useBaseUrl'
import useDocusaurusContext from '@docusaurus/useDocusaurusContext'
import { useThemeConfig } from '@docusaurus/theme-common'
import ThemedImage from '@theme/ThemedImage'

function LogoThemedImage({ logo, alt, imageClassName }) {
  const sources = {
    light: useBaseUrl(logo.src),
    dark: useBaseUrl(logo.srcDark || logo.src),
  }
  const themedImage = (
    <ThemedImage
      className={logo.className}
      sources={sources}
      height={logo.height}
      width={logo.width}
      alt={alt}
      style={logo.style}
    />
  )

  return imageClassName ? (
    <div className={imageClassName}>{themedImage}</div>
  ) : (
    themedImage
  )
}

export default function Logo(props) {
  const {
    siteConfig: { title },
    i18n: { currentLocale },
  } = useDocusaurusContext()
  const {
    navbar: { title: navbarTitle, logo },
  } = useThemeConfig()
  const { imageClassName, titleClassName, ...propsRest } = props
  const logoLink = useBaseUrl(logo?.href || '/')
  const fallbackAlt = navbarTitle ? '' : title
  const alt = logo?.alt ?? fallbackAlt
  const content = (
    <>
      {logo && (
        <LogoThemedImage
          logo={logo}
          alt={alt}
          imageClassName={imageClassName}
        />
      )}
      {navbarTitle != null && <b className={titleClassName}>{navbarTitle}</b>}
    </>
  )

  return (
    <Link
      to={logoLink}
      {...propsRest}
      {...(logo?.target && { target: logo.target })}
    >
      {content}
    </Link>
  )
}
