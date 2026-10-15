+++
id = "01M4V7WCVKFGXHBZGDBBB49Z35"
title = "csv read: R016 error for unbound-name row; # comment only at record start"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T18:20:02Z"
updated = "2026-10-15T03:38:11Z"
scope = ["crates/cadetta-adapter/**"]

[[acceptance]]
text = "A csv row whose name matches no binding is R016 and yields no observed values"
bound = true

[[acceptance]]
text = "A # comment is recognised only at record start"
bound = false
+++

found while working ~6NZV18E. Implement per csv.md 1 and 3.
