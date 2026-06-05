BEGIN { RS="\n## Reflex "; ORS="" }
NR == 1 { next }
{
    block = $0
    name = $1
    gsub(/^[ \t]+|[ \t]+$/, "", name)
    
    if (match(block, /\*\*Trigger:\*\* +[^\n]+/)) {
        trigger = substr(block, RSTART, RLENGTH)
        gsub(/^\*\*Trigger:\*\* +/, "", trigger)
        gsub(/\*\*/, "", trigger)
        gsub(/`/, "", trigger)
        gsub(/^[ \t]+|[ \t]+$/, "", trigger)
    } else { trigger = "" }
    
    if (match(block, /\*\*Reflex:\*\*/)) {
        after = substr(block, RSTART + RLENGTH)
        gsub(/\n\*\*(Taxonomy|Why it works):\*\*.*/, "", after)
        gsub(/\n/, " ", after)
        gsub(/```/, " ", after)
        gsub(/  +/, " ", after)
        gsub(/^[ \t]+|[ \t]+$/, "", after)
        action = after
    } else { action = "" }
    
    print name "|||" trigger "|||" action "\n"
}
