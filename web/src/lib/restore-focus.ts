import * as React from "react"

type Handler = (event: Event) => void

/**
 * Radix only returns focus to a `<Dialog.Trigger>`. Most dialogs here are opened
 * from a controlled `open` state, so without this keyboard and screen-reader
 * users lose their place when a dialog closes. Remember what had focus when the
 * dialog opened and give it back on close (unless the caller prevents it).
 */
export function useRestoreFocus(onOpenAutoFocus?: Handler, onCloseAutoFocus?: Handler) {
  const opener = React.useRef<HTMLElement | null>(null)

  const handleOpen = React.useCallback(
    (event: Event) => {
      opener.current =
        document.activeElement instanceof HTMLElement ? document.activeElement : null
      onOpenAutoFocus?.(event)
    },
    [onOpenAutoFocus],
  )

  const handleClose = React.useCallback(
    (event: Event) => {
      onCloseAutoFocus?.(event)
      if (event.defaultPrevented) return
      const el = opener.current
      if (el && el.isConnected) {
        event.preventDefault()
        el.focus()
      }
    },
    [onCloseAutoFocus],
  )

  return { onOpenAutoFocus: handleOpen, onCloseAutoFocus: handleClose }
}
