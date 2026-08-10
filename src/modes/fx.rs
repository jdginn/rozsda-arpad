pub struct ReaEQ {}
pub struct ReaComp {}

// TODO: we need more smarts to understand "enum" parameters
// Probably fx dump could automatically find them using an algorithm like this:
// - Check whether we have a formatted_value (usually we do)
// - Check whether we have a step size. If it is 1, could be an enum.
// - Check min/max. If they are ints and the range is smallish, could be an enum.
// - Step through min to max at step size and see how the formatted_value changes. If it looks like
// an enum, it probably is one
// - What's the criteria for "looks like an enum?" Maybe check whether anything non-numeric
// changes across all the settings we see? It could still be an enum and we miss it, but the odds
// that our enum-y thing is just changing <some_enum_param>_i would mean that we can just treat it
// as a number with step size anyway.
