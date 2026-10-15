`sync` and `import` now kill an external adapter that leaves a call unanswered past `[[adapter]] timeout` seconds (default 60, shared with `check`) and fail with R008.
