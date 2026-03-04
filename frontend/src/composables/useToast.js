import { ref } from 'vue';

export function useToast() {
    const toast = ref({
        show: false,
        message: '',
        type: 'error'
    });

    const showToast = (message, type = 'error') => {
        toast.value = { show: true, message, type };
        setTimeout(() => {
            toast.value.show = false;
        }, 3000);
    };

    return {
        toast,
        showToast
    };
}
