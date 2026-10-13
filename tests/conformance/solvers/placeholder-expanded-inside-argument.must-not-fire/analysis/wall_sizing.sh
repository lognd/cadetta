cp "${1#--in=}" request.json
printf '%s' '{"schema":"cadetta.solve/1","outputs":{"min_wall":"2mm"}}' > "${2#--out=}"
