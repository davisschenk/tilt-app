import { useMutation, useQueryClient } from "@tanstack/react-query";
import { uploadAttachment, deleteAttachment } from "@/lib/api";

export function useUploadAttachment(brewId: string, eventId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (file: File) => uploadAttachment(brewId, eventId, file),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brew-events", brewId] });
    },
  });
}

export function useDeleteAttachment(brewId: string, eventId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (attachmentId: string) =>
      deleteAttachment(brewId, eventId, attachmentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brew-events", brewId] });
    },
  });
}
