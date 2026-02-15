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
import { useDeleteHydrometer } from "@/hooks/use-hydrometers";
import * as toast from "@/lib/toast";

interface DeleteHydrometerDialogProps {
  hydrometerId: string;
  hydrometerColor: string;
  hydrometerName?: string | null;
  hasActiveBrew: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function DeleteHydrometerDialog({
  hydrometerId,
  hydrometerColor,
  hydrometerName,
  hasActiveBrew,
  open,
  onOpenChange,
}: DeleteHydrometerDialogProps) {
  const deleteHydrometer = useDeleteHydrometer();

  function handleConfirm() {
    deleteHydrometer.mutate(hydrometerId, {
      onSuccess: () => {
        toast.success("Hydrometer deleted");
        onOpenChange(false);
      },
      onError: () => {
        toast.error("Failed to delete hydrometer");
      },
    });
  }

  const displayName = hydrometerName
    ? `${hydrometerColor} (${hydrometerName})`
    : hydrometerColor;

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete "{displayName}" hydrometer?</AlertDialogTitle>
          <AlertDialogDescription>
            {hasActiveBrew ? (
              <>
                <span className="font-medium text-destructive">
                  Warning: This hydrometer is assigned to an active brew.
                </span>{" "}
                Deleting it will unlink it from the brew. This action cannot be
                undone.
              </>
            ) : (
              "This action cannot be undone. This will permanently delete the hydrometer and its configuration."
            )}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleConfirm}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {deleteHydrometer.isPending ? "Deleting..." : "Delete"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
