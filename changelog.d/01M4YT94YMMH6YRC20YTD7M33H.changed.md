The CLI sync path now reads the on-disk csv with read_bound before re-emitting, so an unbound-name row is reported as R016 and exits 1.
