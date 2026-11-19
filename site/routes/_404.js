export const state = () => ({
  get requestedPath() {
    return decodeURI(location.pathname)
  },
})
