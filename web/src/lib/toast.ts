import { toast } from "sonner";

export function success(message: string) {
  toast.success(message, { duration: 4000 });
}

export function error(message: string) {
  toast.error(message, { duration: 4000 });
}

export function info(message: string) {
  toast.info(message, { duration: 4000 });
}
