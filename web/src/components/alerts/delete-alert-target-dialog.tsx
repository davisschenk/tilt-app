import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { useDeleteAlertTarget } from "@/hooks/use-alert-targets";
import * as toast from "@/lib/toast";

interface DeleteAlertTargetDialogProps {
  targetId: string;
  targetName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function DeleteAlertTargetDialog({
  targetId,
  targetName,
  open,
  onOpenChange,
}: DeleteAlertTargetDialogProps) {
  const deleteTarget = useDeleteAlertTarget();

  function handleConfirm() {
    deleteTarget.mutate(targetId, {
      onSuccess: () => {
        toast.success("Alert target deleted");
        onOpenChange(false);
      },
      onError: () => {
        toast.error("Failed to delete alert target");
      },
    });
  }

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete "{targetName}"?</AlertDialogTitle>
          <AlertDialogDescription>
            This will permanently delete this alert target and all associated
            alert rules. This action cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleConfirm}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {deleteTarget.isPending ? "Deleting..." : "Delete"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
