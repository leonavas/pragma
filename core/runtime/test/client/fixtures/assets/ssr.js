self.__ssrModuleRan = true

export default function ($) {
  $.n = 0
  $.open = false

  function inc() { $.n++ }

  return { inc }
}
