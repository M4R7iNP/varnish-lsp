sub vcl_recv {
if
(
req.http.host
==
"www.vg.no"
||
req.http.host
==
"e24.no"
||
req.http.host
==
"www.tek.no"
)
{
set req.backend_hint = my_very_cool_newspaper.backend();
}
else if
(req.http.host == "darthvader.no")
{ set req.backend_hint = lol.backend(); } else {
set req.backend_hint = notfound.backend();
}
}
