import { format } from '/assets/utils/fmt.js'
; export default function ($, $pragma) {

$.open = false
$.items = []

function toggle() {
  $.open = !$.open
  $pragma.log.info(format($pragma.shared.cart.total))
}

;return { toggle }
}
