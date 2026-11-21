export default function ($, $pragma) {
  function mount() {
    self.__net = $pragma
  }

  return { mount }
}
