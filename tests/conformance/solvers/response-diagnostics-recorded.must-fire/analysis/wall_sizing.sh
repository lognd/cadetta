cat > request.json
echo run >> marker
printf '%s' '{"schema":"cadetta.solve/1","diagnostics":[{"severity":"info","message":"converged in 14 iterations"}],"outputs":{"min_wall":"2mm"}}'
