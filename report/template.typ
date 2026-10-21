// Template for lab report
//
// Usage:
//   #import "template.typ": *
//   #show: report.with(title: "Звіт з виконання компʼютерного практикуму №1")

#let report(
  title: none,
  group: none,
  author: none,
  body,
) = {
  set document(title: title, author: author)
  set page(
    paper: "a4",
    margin: (top: 2cm, bottom: 2cm, left: 3cm, right: 1cm),
    numbering: "1",
  )
  set text(
    font: "Times New Roman",
    size: 14pt,
    lang: "uk",
    top-edge: "ascender",
    bottom-edge: "descender",
  )
  set par(
    justify: true,
    leading: 0.65em,
    spacing: 0.65em,
    first-line-indent: (amount: 1.25cm, all: true),
  )

  show heading: set text(size: 14pt)
  show heading: set block(above: 1.4em, below: 1em)

  show raw: set text(font: ("Menlo", "DejaVu Sans Mono"), size: 11pt)
  show raw.where(block: true): set par(justify: false)

  set figure.caption(separator: [ — ])

  align(center, text(weight: "bold", title))
  block(below: 1.4em, par(first-line-indent: 0pt)[Виконав студент групи #group #author])

  body
}
